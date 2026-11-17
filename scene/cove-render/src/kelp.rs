//! The kelp pass: a frame of long ribbon blades at the picture's left and right edges, drawn into the scene
//! target with depth test and write. One mesh (a ribbon strip) is drawn once per blade; the vertex shader
//! (`shaders/kelp.wgsl`, adapted from abyssal-garden's `W_LEAF`) bends and moves it.

use cove_scene::scene::SceneFrame;

use crate::targets::{COLOR_FORMAT, DEPTH_FORMAT};

/// Rows along a blade: enough that a blade seen 2 m away stays smooth. Must match `ROWS` in `kelp.wgsl`.
const BLADE_ROWS: usize = 48;
/// Bytes of one blade in the instance buffer: three `vec4f`.
const BLADE_STRIDE: u64 = 48;

/// The mesh of one blade: a ribbon strip of `BLADE_ROWS` rows, two vertices each. Each vertex is
/// (u, row): u is -1 or 1 across the blade, row runs 0..BLADE_ROWS-1 from root to tip.
pub(crate) fn build_mesh() -> (Vec<[f32; 2]>, Vec<u16>) {
    let mut vertices = Vec::with_capacity(BLADE_ROWS * 2);
    for row in 0..BLADE_ROWS {
        vertices.push([-1.0, row as f32]);
        vertices.push([1.0, row as f32]);
    }
    let mut indices = Vec::with_capacity((BLADE_ROWS - 1) * 6);
    for row in 0..BLADE_ROWS - 1 {
        let a = (row * 2) as u16;
        indices.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
    }
    (vertices, indices)
}

pub(crate) struct Kelp {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    blades: wgpu::Buffer,
    /// How many blades `blades` has room for.
    capacity: usize,
    uniforms: wgpu::Buffer,
    group: wgpu::BindGroup,
    count: u32,
}

impl Kelp {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let source = [
            include_str!("../shaders/common.wgsl"),
            include_str!("../shaders/kelp.wgsl"),
        ]
        .concat();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kelp"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let mesh_attributes = [wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 0,
            shader_location: 0,
        }];
        let blade_attributes = [1u32, 2, 3].map(|location| wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 16 * (location as u64 - 1),
            shader_location: location,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kelp"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("kelp_vertex"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 8,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &mesh_attributes,
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: BLADE_STRIDE,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &blade_attributes,
                    }),
                ],
            },
            // Blades are seen from both sides.
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
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kelp"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let capacity = 16;
        let blades = Self::blade_buffer(device, capacity);
        Kelp {
            pipeline,
            vertices,
            indices,
            index_count: index_data.len() as u32,
            blades,
            capacity,
            uniforms,
            group,
            count: 0,
        }
    }

    fn blade_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kelp blades"),
            size: capacity as u64 * BLADE_STRIDE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Uploads the frame's blades and uniforms. Call before submitting the encoder that holds
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
            self.blades = Self::blade_buffer(device, self.capacity);
        }
        let blades: Vec<[f32; 4]> = frame
            .kelp
            .iter()
            .flat_map(|b| {
                [
                    [
                        b.root[0] as f32,
                        b.root[1] as f32,
                        b.root[2] as f32,
                        b.length as f32,
                    ],
                    [
                        b.lean as f32,
                        b.forward as f32,
                        b.width as f32,
                        b.curvature as f32,
                    ],
                    [
                        b.seed[0] as f32,
                        b.seed[1] as f32,
                        b.side as f32,
                        b.sway_phase as f32,
                    ],
                ]
            })
            .collect();
        queue.write_buffer(&self.blades, 0, bytemuck::cast_slice(&blades));

        let sky = &frame.sky;
        let m = &frame.kelp_motion;
        let v = |a: [f64; 3], w: f64| [a[0] as f32, a[1] as f32, a[2] as f32, w as f32];
        let uniforms = [
            [
                (size.0 as f64 / size.1 as f64) as f32,
                m.surge as f32,
                m.time as f32,
                0.0,
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
        pass.set_vertex_buffer(1, self.blades.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.index_count, 0, 0..self.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_is_one_ribbon() {
        let (vertices, indices) = build_mesh();
        assert_eq!(vertices.len(), 2 * BLADE_ROWS);
        assert_eq!(indices.len(), (BLADE_ROWS - 1) * 6);
        assert!(indices.iter().all(|&i| (i as usize) < vertices.len()));
        assert_eq!(vertices[3], [1.0, 1.0]);
    }
}
