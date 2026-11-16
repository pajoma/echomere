//! The kelp pass: one mesh (a stalk and 64 blades) drawn once per frond into the scene target, with depth
//! test and write. Port of `KelpMesh.swift` and the kelp draw of `CoveSceneRenderer.swift`; the shaders are
//! `shaders/common.wgsl` and `shaders/kelp.wgsl`.

use cove_scene::scene::SceneFrame;

use crate::ocean::Ocean;
use crate::targets::{COLOR_FORMAT, DEPTH_FORMAT};

/// Segments along the stalk ribbon.
const STALK_SEGMENTS: usize = 48;
/// Must match `BLADES` in `kelp.wgsl`.
const BLADES: usize = 64;
const BLADE_ROWS: usize = 9;
/// Bytes of one frond in the instance buffer: two `vec4f`.
const FROND_STRIDE: u64 = 32;

/// The mesh of one kelp stalk with its blades. The vertices only say where on the stalk they belong; the
/// vertex shader puts them in the water, so one mesh serves every stalk. Each vertex is (s, u, v, blade):
/// - stalk: s runs 0..1 from root to tip, v is -1 or 1 across the ribbon, blade is -1;
/// - blade: u runs 0..1 from base to tip, v runs -1..1 across, blade is the blade's index.
pub(crate) fn build_mesh() -> (Vec<[f32; 4]>, Vec<u16>) {
    let mut vertices: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();

    for i in 0..=STALK_SEGMENTS {
        let s = i as f32 / STALK_SEGMENTS as f32;
        vertices.push([s, 0.0, -1.0, -1.0]);
        vertices.push([s, 0.0, 1.0, -1.0]);
    }
    for i in 0..STALK_SEGMENTS {
        let a = (i * 2) as u16;
        indices.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
    }

    for blade in 0..BLADES {
        let first = vertices.len() as u16;
        for row in 0..BLADE_ROWS {
            let u = row as f32 / (BLADE_ROWS - 1) as f32;
            for v in [-1.0f32, 0.0, 1.0] {
                vertices.push([0.0, u, v, blade as f32]);
            }
        }
        for row in 0..BLADE_ROWS - 1 {
            for column in 0..2 {
                let a = first + (row * 3 + column) as u16;
                indices.extend_from_slice(&[a, a + 1, a + 3, a + 1, a + 4, a + 3]);
            }
        }
    }
    (vertices, indices)
}

pub(crate) struct Kelp {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    fronds: wgpu::Buffer,
    /// How many fronds `fronds` has room for.
    capacity: usize,
    uniforms: wgpu::Buffer,
    group: wgpu::BindGroup,
    count: u32,
}

impl Kelp {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, ocean: &Ocean) -> Self {
        let source = [
            include_str!("../shaders/common.wgsl"),
            include_str!("../shaders/kelp.wgsl"),
        ]
        .concat();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kelp"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let float4 = |location| wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 0,
            shader_location: location,
        };
        let mesh_attributes = [float4(0)];
        let frond_attributes = [
            float4(1),
            wgpu::VertexAttribute {
                offset: 16,
                ..float4(2)
            },
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kelp"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("kelp_vertex"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 16,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &mesh_attributes,
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: FROND_STRIDE,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &frond_attributes,
                    }),
                ],
            },
            // Blades and the stalk ribbon are seen from both sides.
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
                entry_point: Some("kelp_fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let (mesh, index_data) = build_mesh();
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
            "kelp vertices",
            wgpu::BufferUsages::VERTEX,
            bytemuck::cast_slice(&mesh),
        );
        let indices = buffer(
            "kelp indices",
            wgpu::BufferUsages::INDEX,
            bytemuck::cast_slice(&index_data),
        );

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kelp uniforms"),
            size: 4 * 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("kelp sea"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let displacement = ocean
            .displacement
            .create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kelp"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&displacement),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let capacity = 64;
        let fronds = Self::frond_buffer(device, capacity);
        Kelp {
            pipeline,
            vertices,
            indices,
            index_count: index_data.len() as u32,
            fronds,
            capacity,
            uniforms,
            group,
            count: 0,
        }
    }

    fn frond_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kelp fronds"),
            size: capacity as u64 * FROND_STRIDE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Uploads the frame's fronds and uniforms. Call before submitting the encoder that holds
    /// [`Kelp::draw`]. `size` is the scene target's size in pixels;
    /// nothing is drawn when `visible` is false.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &SceneFrame,
        size: (u32, u32),
        visible: bool,
    ) {
        self.count = if visible { frame.kelp.len() as u32 } else { 0 };
        if self.count == 0 {
            return;
        }
        if frame.kelp.len() > self.capacity {
            self.capacity = frame.kelp.len().next_power_of_two();
            self.fronds = Self::frond_buffer(device, self.capacity);
        }
        let mut fronds: Vec<[f32; 4]> = Vec::with_capacity(frame.kelp.len() * 2);
        for f in &frame.kelp {
            fronds.push([
                f.base_x as f32,
                f.base_depth as f32,
                f.base_z as f32,
                f.length as f32,
            ]);
            fronds.push([
                f.sway_phase as f32,
                f.twist_phase as f32,
                f.blade_seed as f32,
                f.blade_scale as f32,
            ]);
        }
        queue.write_buffer(&self.fronds, 0, bytemuck::cast_slice(&fronds));

        let sky = &frame.sky;
        let m = &frame.kelp_motion;
        let v = |a: [f64; 3], w: f64| [a[0] as f32, a[1] as f32, a[2] as f32, w as f32];
        let uniforms = [
            [
                (size.0 as f64 / size.1 as f64) as f32,
                m.surge as f32,
                m.drift as f32,
                m.flutter as f32,
            ],
            [1.0 / size.1 as f32, sky.warmth as f32, 0.0, 0.0],
            v(sky.light_direction, sky.light_intensity),
            v(sky.light_colour, sky.ambient),
        ];
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&uniforms));
    }

    /// Draws the kelp into the scene pass, after the water.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, self.fronds.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.index_count, 0, 0..self.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_has_the_swift_layout() {
        let (vertices, indices) = build_mesh();
        assert_eq!(vertices.len(), 2 * 49 + 64 * 27);
        assert_eq!(indices.len(), 48 * 6 + 64 * 8 * 2 * 6);
        assert!(indices.iter().all(|&i| (i as usize) < vertices.len()));
        assert_eq!(vertices[2], [1.0 / 48.0, 0.0, -1.0, -1.0]);
        assert_eq!(vertices[98 + 3], [0.0, 1.0 / 8.0, -1.0, 0.0]);
    }
}
