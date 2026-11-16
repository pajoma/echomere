//! The whales: one model per species (mesh, skin and swim animation), drawn once per whale into the scene
//! target after the kelp, with depth test and write, blended by the whale's opacity. Port of
//! `WhaleModel.swift` and the whale draw of `CoveSceneRenderer.swift`; the shaders are `shaders/common.wgsl`,
//! `shaders/caustics.wgsl` and `shaders/whale.wgsl`.
//!
//! The model files are made by `tools/prepare-whales.py` (see it for the file formats) and live in
//! `assets/whales/prepared/`, under CC BY-NC 4.0 (their `LICENSE.md`).

use std::path::{Path, PathBuf};

use cove_scene::model::WhaleSpecies;
use cove_scene::scene::SceneFrame;

use crate::gpu::RenderError;
use crate::ocean::Ocean;
use crate::targets::{COLOR_FORMAT, DEPTH_FORMAT};

/// The most joints a model may have: their matrices are sent to the shader with every whale. Must match
/// `MAX_JOINTS` in `whale.wgsl`.
pub const MAX_JOINTS: usize = 80;
/// `vec4f` before the joint matrices in a whale's uniforms. Must match `WHALE_HEADER` in `whale.wgsl`.
const HEADER: usize = 6;
/// Bytes of one whale's uniforms.
const WHALE_BYTES: u64 = ((HEADER + 3 * MAX_JOINTS) * 16) as u64;
/// Position (3 floats), normal (3 floats), texture coordinate (2 floats), joints (4 uint16), weights (4
/// floats).
const VERTEX_STRIDE: usize = 8 * 4 + 4 * 2 + 4 * 4;

/// The folder the shared assets are read from: `$ECHOMERE_ASSETS`, otherwise `assets` next to the
/// executable, otherwise the repository's `assets` (for runs from the workspace, such as the tests).
pub fn assets_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("ECHOMERE_ASSETS") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("assets")))
    {
        if dir.is_dir() {
            return dir;
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

/// The folder of the prepared whale models inside the assets.
pub fn whales_dir() -> PathBuf {
    assets_dir().join("whales").join("prepared")
}

/// The file name (without extension) of a species' model.
pub fn file_name(species: WhaleSpecies) -> &'static str {
    match species {
        WhaleSpecies::SouthernRightWhale => "southern-right-whale",
        WhaleSpecies::Orca => "orca",
    }
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("four bytes"))
}

/// A model's own swim animation: for every frame, every joint's matrix (three rows of four) from the bind
/// pose to the swimming whale. It loops.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Animation {
    joints: usize,
    frame_count: usize,
    frames: Vec<[f32; 4]>,
    /// The average pose over the loop: what the whale looks like when it glides.
    glide: Vec<[f32; 4]>,
}

impl Animation {
    /// Reads an animation file ("EWA1"); the error says what is wrong.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 16 || &bytes[..4] != b"EWA1" {
            return Err("not an animation file".into());
        }
        let joints = u32_at(bytes, 4) as usize;
        let frame_count = u32_at(bytes, 8) as usize;
        if joints == 0 || joints > MAX_JOINTS || frame_count == 0 {
            return Err("wrong animation size".into());
        }
        let expected = joints
            .checked_mul(frame_count)
            .and_then(|n| n.checked_mul(3 * 16))
            .and_then(|n| n.checked_add(16));
        if expected != Some(bytes.len()) {
            return Err("wrong animation size".into());
        }
        let frames: Vec<[f32; 4]> = bytes[16..]
            .chunks_exact(16)
            .map(|row| {
                let f = |i: usize| {
                    f32::from_le_bytes(row[i * 4..i * 4 + 4].try_into().expect("four bytes"))
                };
                [f(0), f(1), f(2), f(3)]
            })
            .collect();
        let mut glide = vec![[0.0f32; 4]; joints * 3];
        for frame in 0..frame_count {
            for (row, sum) in glide.iter_mut().enumerate() {
                for i in 0..4 {
                    sum[i] += frames[frame * joints * 3 + row][i];
                }
            }
        }
        for row in &mut glide {
            for v in row.iter_mut() {
                *v /= frame_count as f32;
            }
        }
        Ok(Animation {
            joints,
            frame_count,
            frames,
            glide,
        })
    }

    /// The joint matrices (three rows per joint) for a moment in the swim loop. `phase` is where in the
    /// loop, 0..1; `effort` 1 swims fully, towards 0 the whale glides, holding the loop's average pose.
    pub fn pose(&self, phase: f64, effort: f64) -> Vec<[f32; 4]> {
        let position = (phase - phase.floor()) * self.frame_count as f64;
        let first = position as usize % self.frame_count;
        let second = (first + 1) % self.frame_count;
        let blend = (position - position.floor()) as f32;
        let strength = (0.25 + 0.75 * effort.clamp(0.0, 1.0)) as f32;
        let rows = self.joints * 3;
        (0..rows)
            .map(|row| {
                let (a, b, g) = (
                    self.frames[first * rows + row],
                    self.frames[second * rows + row],
                    self.glide[row],
                );
                std::array::from_fn(|i| g[i] + (a[i] + (b[i] - a[i]) * blend - g[i]) * strength)
            })
            .collect()
    }
}

/// A whale model on the GPU: its mesh, its skin texture and its swim animation.
#[derive(Debug)]
pub struct WhaleModel {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    skin: wgpu::TextureView,
    animation: Animation,
}

impl WhaleModel {
    /// Loads `<name>.mesh`, `<name>.animation` and `<name>.jpg` of `species` from `dir`.
    pub fn load(
        dir: &Path,
        species: WhaleSpecies,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self, RenderError> {
        let name = file_name(species);
        let read = |extension: &str| {
            let path = dir.join(format!("{name}.{extension}"));
            std::fs::read(&path).map_err(|e| RenderError::Asset(format!("{}: {e}", path.display())))
        };
        let bad = |what: &str, problem: String| {
            RenderError::Asset(format!(
                "{}: {problem}",
                dir.join(format!("{name}.{what}")).display()
            ))
        };

        let mesh = read("mesh")?;
        if mesh.len() < 12 || &mesh[..4] != b"EWM2" {
            return Err(bad("mesh", "not a mesh file".into()));
        }
        let vertex_count = u32_at(&mesh, 4) as usize;
        let index_count = u32_at(&mesh, 8) as usize;
        let vertex_bytes = vertex_count * VERTEX_STRIDE;
        if mesh.len() != 12 + vertex_bytes + index_count * 4 {
            return Err(bad("mesh", "wrong size".into()));
        }
        let animation = Animation::parse(&read("animation")?).map_err(|e| bad("animation", e))?;
        let skin = image::load_from_memory(&read("jpg")?)
            .map_err(|e| bad("jpg", e.to_string()))?
            .to_rgba8();

        let buffer = |label, usage, contents: &[u8]| {
            let b = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (contents.len() as u64).next_multiple_of(4),
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&b, 0, contents);
            b
        };
        let vertices = buffer(
            "whale vertices",
            wgpu::BufferUsages::VERTEX,
            &mesh[12..12 + vertex_bytes],
        );
        let indices = buffer(
            "whale indices",
            wgpu::BufferUsages::INDEX,
            &mesh[12 + vertex_bytes..],
        );
        Ok(WhaleModel {
            vertices,
            indices,
            index_count: index_count as u32,
            skin: upload_skin(device, queue, &skin),
            animation,
        })
    }

    /// The joint matrices for a moment in the swim loop; see [`Animation::pose`].
    pub fn pose(&self, phase: f64, effort: f64) -> Vec<[f32; 4]> {
        self.animation.pose(phase, effort)
    }
}

/// Uploads the skin with mipmaps (as the Swift renderer's texture loader made them). The texture is not
/// sRGB: the shader decodes the skin with `to_linear`.
fn upload_skin(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    skin: &image::RgbaImage,
) -> wgpu::TextureView {
    let (width, height) = skin.dimensions();
    let levels = 32 - width.max(height).leading_zeros();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("whale skin"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut level_image = skin.clone();
    for level in 0..levels {
        let (w, h) = ((width >> level).max(1), (height >> level).max(1));
        if level > 0 {
            level_image =
                image::imageops::resize(&level_image, w, h, image::imageops::FilterType::Triangle);
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            level_image.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// One whale to draw this frame: which model, and where its uniforms are.
struct Draw {
    model: usize,
    offset: u32,
}

pub(crate) struct Whales {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    curvature: wgpu::TextureView,
    /// One per species, in the order of [`WhaleSpecies::ALL`].
    models: Vec<WhaleModel>,
    /// Every whale's uniforms of the frame, `slot` bytes apart.
    uniforms: wgpu::Buffer,
    slot: u64,
    /// How many whales `uniforms` has room for.
    capacity: usize,
    /// One bind group per model, on `uniforms`.
    groups: Vec<wgpu::BindGroup>,
    draws: Vec<Draw>,
}

impl Whales {
    /// Loads both species' models from `dir`.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        ocean: &Ocean,
        dir: &Path,
    ) -> Result<Self, RenderError> {
        let models = WhaleSpecies::ALL
            .iter()
            .map(|&species| WhaleModel::load(dir, species, device, queue))
            .collect::<Result<Vec<_>, _>>()?;

        let source = [
            include_str!("../shaders/common.wgsl"),
            include_str!("../shaders/caustics.wgsl"),
            include_str!("../shaders/whale.wgsl"),
        ]
        .concat();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("whale"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let texture = |binding, dimension| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: dimension,
                multisampled: false,
            },
            count: None,
        };
        // The uniforms have a dynamic offset: one buffer holds every whale of the frame.
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("whale"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(WHALE_BYTES),
                    },
                    count: None,
                },
                texture(1, wgpu::TextureViewDimension::D2),
                texture(2, wgpu::TextureViewDimension::D2Array),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("whale"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let attribute = |format, offset, shader_location| wgpu::VertexAttribute {
            format,
            offset,
            shader_location,
        };
        let attributes = [
            attribute(wgpu::VertexFormat::Float32x3, 0, 0),
            attribute(wgpu::VertexFormat::Float32x3, 12, 1),
            attribute(wgpu::VertexFormat::Float32x2, 24, 2),
            attribute(wgpu::VertexFormat::Uint16x4, 32, 3),
            attribute(wgpu::VertexFormat::Float32x4, 40, 4),
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("whale"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("whale_vertex"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: VERTEX_STRIDE as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                })],
            },
            // Seen from both sides, as in the Swift renderer: some models are not closed.
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("whale_fragment"),
                compilation_options: Default::default(),
                // A whale fades in when it arrives: straight alpha over the linear light behind it.
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("whale"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let curvature = ocean.curvature.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let mut whales = Whales {
            pipeline,
            layout,
            sampler,
            curvature,
            models,
            uniforms: device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 4,
                usage: wgpu::BufferUsages::UNIFORM,
                mapped_at_creation: false,
            }),
            slot: WHALE_BYTES.next_multiple_of(alignment),
            capacity: 0,
            groups: Vec::new(),
            draws: Vec::new(),
        };
        whales.grow(device, 4);
        Ok(whales)
    }

    /// Makes room for `capacity` whales.
    fn grow(&mut self, device: &wgpu::Device, capacity: usize) {
        self.capacity = capacity;
        self.uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("whale uniforms"),
            size: self.slot * capacity as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.groups = self
            .models
            .iter()
            .map(|model| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("whale"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &self.uniforms,
                                offset: 0,
                                size: wgpu::BufferSize::new(WHALE_BYTES),
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&model.skin),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&self.curvature),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                })
            })
            .collect();
    }

    /// Uploads every visible whale's uniforms. Call before submitting the encoder that holds
    /// [`Whales::draw`]. `size` is the scene target's size in pixels; nothing is drawn when `visible` is
    /// false.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &SceneFrame,
        size: (u32, u32),
        visible: bool,
    ) {
        self.draws.clear();
        if !visible {
            return;
        }
        let whales: Vec<_> = frame.whales.iter().filter(|w| w.opacity > 0.0).collect();
        if whales.len() > self.capacity {
            self.grow(device, whales.len().next_power_of_two());
        }
        let sky = &frame.sky;
        let v = |a: [f64; 3], w: f64| [a[0] as f32, a[1] as f32, a[2] as f32, w as f32];
        let mut data = vec![0u8; self.slot as usize * whales.len()];
        for (i, whale) in whales.iter().enumerate() {
            let model = WhaleSpecies::ALL
                .iter()
                .position(|&s| s == whale.species)
                .expect("every species has a model");
            let mut uniforms = vec![
                v(whale.position, whale.length),
                v(whale.forward, whale.roll),
                [0.0, whale.opacity as f32, 0.0, 0.0],
                [size.0 as f32 / size.1 as f32, 1.0 / size.1 as f32, 0.0, 0.0],
                v(sky.light_direction, sky.light_intensity),
                v(sky.light_colour, sky.ambient),
            ];
            // The model's own swim animation, at the point of the whale's tail beat.
            uniforms.extend(
                self.models[model].pose(whale.tail_phase / std::f64::consts::TAU, whale.effort),
            );
            let bytes: &[u8] = bytemuck::cast_slice(&uniforms);
            let start = i * self.slot as usize;
            data[start..start + bytes.len()].copy_from_slice(bytes);
            self.draws.push(Draw {
                model,
                offset: start as u32,
            });
        }
        if !data.is_empty() {
            queue.write_buffer(&self.uniforms, 0, &data);
        }
    }

    /// Draws the whales into the scene pass, after the kelp.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.draws.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        for draw in &self.draws {
            let model = &self.models[draw.model];
            pass.set_bind_group(0, &self.groups[draw.model], &[draw.offset]);
            pass.set_vertex_buffer(0, model.vertices.slice(..));
            pass.set_index_buffer(model.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..model.index_count, 0, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An animation file of `joints` joints and `frames` frames whose every value is its frame number.
    fn animation_file(joints: u32, frames: u32) -> Vec<u8> {
        let mut bytes = b"EWA1".to_vec();
        bytes.extend(joints.to_le_bytes());
        bytes.extend(frames.to_le_bytes());
        bytes.extend(1.0f32.to_le_bytes());
        for frame in 0..frames {
            for _ in 0..joints * 12 {
                bytes.extend((frame as f32).to_le_bytes());
            }
        }
        bytes
    }

    #[test]
    fn pose_blends_frames_and_glides_like_swift() {
        let a = Animation::parse(&animation_file(2, 4)).unwrap();
        // Halfway between frames 1 and 2, swimming fully.
        assert_eq!(a.pose(0.375, 1.0), vec![[1.5; 4]; 6]);
        // Gliding keeps a quarter of the movement around the average pose (1.5).
        assert_eq!(a.pose(0.0, 0.0), vec![[1.5 + (0.0 - 1.5) * 0.25; 4]; 6]);
        // The loop wraps: after the last frame comes the first.
        assert_eq!(a.pose(1.875, 1.0), vec![[1.5; 4]; 6]);
    }

    #[test]
    fn bad_animation_files_are_refused() {
        assert!(Animation::parse(b"EWA0").is_err());
        assert!(Animation::parse(&animation_file(MAX_JOINTS as u32 + 1, 1)).is_err());
        let mut short = animation_file(2, 2);
        short.pop();
        assert!(Animation::parse(&short).is_err());
    }

    #[test]
    fn absurd_animation_counts_are_refused_without_overflow() {
        let mut header = b"EWA1".to_vec();
        header.extend(u32::MAX.to_le_bytes());
        header.extend(u32::MAX.to_le_bytes());
        header.extend(1.0f32.to_le_bytes());
        assert!(Animation::parse(&header).is_err());
    }

    #[test]
    fn prepared_models_are_valid() {
        for species in WhaleSpecies::ALL {
            let path = whales_dir().join(format!("{}.animation", file_name(species)));
            let animation = Animation::parse(&std::fs::read(&path).unwrap()).unwrap();
            assert!(
                animation.joints > 0 && animation.frame_count > 0,
                "{}",
                path.display()
            );
        }
    }
}
