//! The FFT ocean on the GPU: its spectrum (computed once), and per frame the displacement, slopes and
//! curvature of the surface for every cascade. Port of `Ocean` and `OceanSettings` in
//! `CoveSceneRenderer.swift`; the shaders are in `shaders/ocean.wgsl`.

use cove_scene::sea;

/// Number of cascades (patch sizes) the sea is simulated in.
pub(crate) const CASCADES: u32 = 3;
/// Resolution of every cascade.
pub(crate) const N: u32 = 256;
/// Mip levels of the curvature texture (the light shafts read a blurred version).
pub(crate) const CURVATURE_MIPS: u32 = 5;
const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// Root-mean-square size of the sea's height, slope and curvature in one cascade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeaStatistics {
    pub cascade: usize,
    pub height_rms: f64,
    pub slope_rms: f64,
    pub curvature_rms: f64,
}

/// Must match `OceanSettings` in `ocean.wgsl`.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct OceanSettings {
    l: [f32; 4],
    k_min: [f32; 4],
    k_max: [f32; 4],
    wind: [f32; 4],
    swell: [f32; 4],
    misc: [f32; 4],
    time_p: [f32; 4],
}

impl OceanSettings {
    pub fn from_sea_time(sea_time: f64) -> Self {
        let n = N as f64;
        let sizes = sea::CASCADE_SIZES;
        let (mut l, mut k_min, mut k_max) = ([1.0f32; 4], [0.0f32; 4], [0.0f32; 4]);
        for c in 0..sizes.len() {
            l[c] = sizes[c] as f32;
            k_min[c] = if c == 0 {
                std::f64::consts::PI / sizes[c]
            } else {
                2.0 * std::f64::consts::PI * 6.0 / sizes[c]
            } as f32;
            k_max[c] = if c < sizes.len() - 1 {
                2.0 * std::f64::consts::PI * 6.0 / sizes[c + 1]
            } else {
                std::f64::consts::PI * n / sizes[c] * 0.98
            } as f32;
        }
        let degrees = std::f64::consts::PI / 180.0;
        let swell_k = 2.0 * std::f64::consts::PI / sea::SWELL_WAVELENGTH;
        OceanSettings {
            l,
            k_min,
            k_max,
            wind: [
                sea::WIND_SPEED as f32,
                (sea::WIND_DIRECTION * degrees) as f32,
                sea::FETCH as f32,
                sea::JONSWAP_GAMMA as f32,
            ],
            swell: [
                sea::SWELL_HEIGHT as f32,
                (sea::SWELL_DIRECTION * degrees) as f32,
                (9.81 * swell_k).sqrt() as f32,
                sea::SWELL_SPREAD as f32,
            ],
            misc: [
                sea::CHOPPINESS as f32,
                sea::SHORT_WAVE_CUT as f32,
                sea::LOOP_PERIOD as f32,
                sea::SEED as f32,
            ],
            time_p: [sea_time as f32, 0.0, 0.0, 1.0],
        }
    }
}

pub(crate) struct Ocean {
    /// Per cascade (array layer): Dx, height, Dz, dDx/dz.
    pub displacement: wgpu::Texture,
    /// Per cascade: slope x, slope z, dDx/dx, dDz/dz.
    pub derivatives: wgpu::Texture,
    /// Per cascade: d2h/dx2, d2h/dxdz, d2h/dz2, with `CURVATURE_MIPS` mip levels.
    pub curvature: wgpu::Texture,
    settings: wgpu::Buffer,
    evolve: (wgpu::ComputePipeline, wgpu::BindGroup),
    rows: (wgpu::ComputePipeline, wgpu::BindGroup),
    columns: (wgpu::ComputePipeline, wgpu::BindGroup),
    curvature_pass: (wgpu::ComputePipeline, wgpu::BindGroup),
    mips: (wgpu::ComputePipeline, Vec<wgpu::BindGroup>),
}

impl Ocean {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ocean"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ocean.wgsl").into()),
        });
        let pipeline = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };

        let length = (CASCADES * N * N) as u64 * 16;
        let buffer = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: length,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };
        let (h0, spec_a, spec_b, rows_a, rows_b) = (
            buffer("h0"),
            buffer("spectrum a"),
            buffer("spectrum b"),
            buffer("rows a"),
            buffer("rows b"),
        );
        let settings = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ocean settings"),
            size: std::mem::size_of::<OceanSettings>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture = |label, mips| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: N,
                    height: N,
                    depth_or_array_layers: CASCADES,
                },
                mip_level_count: mips,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TEXTURE_FORMAT,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        };
        let displacement = texture("ocean displacement", 1);
        let derivatives = texture("ocean derivatives", 1);
        let curvature = texture("ocean curvature", CURVATURE_MIPS);
        let level = |t: &wgpu::Texture, mip: u32| {
            t.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                base_mip_level: mip,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };

        let bind = |pipeline: &wgpu::ComputePipeline, entries: &[(u32, wgpu::BindingResource)]| {
            let entries: Vec<_> = entries
                .iter()
                .map(|(binding, resource)| wgpu::BindGroupEntry {
                    binding: *binding,
                    resource: resource.clone(),
                })
                .collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &entries,
            })
        };

        let init = pipeline("ocean_init");
        let init_group = bind(
            &init,
            &[
                (0, settings.as_entire_binding()),
                (1, h0.as_entire_binding()),
            ],
        );

        let evolve = pipeline("ocean_evolve");
        let evolve_group = bind(
            &evolve,
            &[
                (0, settings.as_entire_binding()),
                (2, h0.as_entire_binding()),
                (3, spec_a.as_entire_binding()),
                (4, spec_b.as_entire_binding()),
            ],
        );
        let rows = pipeline("ocean_fft_rows");
        let rows_group = bind(
            &rows,
            &[
                (5, spec_a.as_entire_binding()),
                (6, spec_b.as_entire_binding()),
                (7, rows_a.as_entire_binding()),
                (8, rows_b.as_entire_binding()),
            ],
        );
        let columns = pipeline("ocean_fft_cols");
        let (displacement_out, derivatives_out) = (level(&displacement, 0), level(&derivatives, 0));
        let columns_group = bind(
            &columns,
            &[
                (5, rows_a.as_entire_binding()),
                (6, rows_b.as_entire_binding()),
                (9, wgpu::BindingResource::TextureView(&displacement_out)),
                (10, wgpu::BindingResource::TextureView(&derivatives_out)),
            ],
        );
        let curvature_pass = pipeline("ocean_curvature");
        let curvature_out = level(&curvature, 0);
        let curvature_group = bind(
            &curvature_pass,
            &[
                (0, settings.as_entire_binding()),
                (11, wgpu::BindingResource::TextureView(&derivatives_out)),
                (12, wgpu::BindingResource::TextureView(&curvature_out)),
            ],
        );
        let mip = pipeline("ocean_curvature_mip");
        let mip_groups = (1..CURVATURE_MIPS)
            .map(|m| {
                let (from, to) = (level(&curvature, m - 1), level(&curvature, m));
                bind(
                    &mip,
                    &[
                        (13, wgpu::BindingResource::TextureView(&from)),
                        (14, wgpu::BindingResource::TextureView(&to)),
                    ],
                )
            })
            .collect();

        // The spectrum never changes: compute it once.
        queue.write_buffer(
            &settings,
            0,
            bytemuck::bytes_of(&OceanSettings::from_sea_time(0.0)),
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&init);
            pass.set_bind_group(0, &init_group, &[]);
            pass.dispatch_workgroups(N / 16, N / 16, CASCADES);
        }
        queue.submit([encoder.finish()]);

        Ocean {
            displacement,
            derivatives,
            curvature,
            settings,
            evolve: (evolve, evolve_group),
            rows: (rows, rows_group),
            columns: (columns, columns_group),
            curvature_pass: (curvature_pass, curvature_group),
            mips: (mip, mip_groups),
        }
    }

    /// Simulates the sea at `sea_time` (seconds within the loop) into the ocean textures.
    pub fn simulate(&self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, sea_time: f64) {
        queue.write_buffer(
            &self.settings,
            0,
            bytemuck::bytes_of(&OceanSettings::from_sea_time(sea_time)),
        );
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ocean"),
            timestamp_writes: None,
        });
        let mut run = |(pipeline, group): &(wgpu::ComputePipeline, wgpu::BindGroup), x, y, z| {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(x, y, z);
        };
        run(&self.evolve, N / 16, N / 16, CASCADES);
        run(&self.rows, N, 1, CASCADES);
        run(&self.columns, N, 1, CASCADES);
        run(&self.curvature_pass, N / 16, N / 16, CASCADES);
        let (mip, groups) = &self.mips;
        pass.set_pipeline(mip);
        for (m, group) in groups.iter().enumerate() {
            let side = N >> (m + 1);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(side.div_ceil(8), side.div_ceil(8), CASCADES);
        }
    }
}

/// IEEE 754 half precision to f32.
pub(crate) fn half_to_f32(bits: u16) -> f32 {
    let sign = ((bits & 0x8000) as u32) << 16;
    let exponent = ((bits >> 10) & 0x1F) as i32;
    let mantissa = (bits & 0x3FF) as u32;
    if exponent == 0 {
        let value = mantissa as f32 / 1024.0 * 2f32.powi(-14);
        return if sign == 0 { value } else { -value };
    }
    if exponent == 31 {
        return f32::from_bits(sign | 0x7F80_0000 | (mantissa << 13));
    }
    f32::from_bits(sign | (((exponent - 15 + 127) as u32) << 23) | (mantissa << 13))
}

/// Root mean square over the cascade `cascade` of `channels`, from a tightly packed `rgba16float`
/// readback of all cascades.
pub(crate) fn rms(halves: &[u16], cascade: usize, channels: &[usize]) -> f64 {
    let texels = (N * N) as usize;
    let mut sum = 0.0;
    for i in 0..texels {
        for &ch in channels {
            let v = half_to_f32(halves[(cascade * texels + i) * 4 + ch]) as f64;
            sum += v * v;
        }
    }
    (sum / texels as f64).sqrt()
}
