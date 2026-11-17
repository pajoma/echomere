//! The fish pass: every fish of the schools drawn as a small 3D fish, one mesh instanced per fish, into the
//! scene target with depth test and write (after the kelp and the whales, so kelp in front covers fish).
//! The vertex shader (`shaders/fish.wgsl`) bends the body with a swimming wave and turns it along the
//! fish's heading.

use cove_scene::scene::SceneFrame;
use cove_scene::schools::FishKind;

use crate::targets::{COLOR_FORMAT, DEPTH_FORMAT};

/// Rings along the body and segments around it.
const RINGS: usize = 12;
const SEGMENTS: usize = 8;
/// Bytes of one fish in the instance buffer: three `vec4f`.
const FISH_STRIDE: u64 = 48;
/// The most fish drawn in one frame (the fish design's capacity).
pub(crate) const MAX_FISH: usize = 512;

/// The fish mesh, one unit long: x runs from the tail (-0.5) to the nose (0.5); y is up and z sideways, both
/// for a body of height and width 1 (the shader scales them per kind). Each vertex is (x, y, z, fin): fin is
/// 1 for the tail fin, 0 for the body.
pub(crate) fn build_mesh() -> (Vec<[f32; 4]>, Vec<u16>) {
    let mut vertices: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    // A spindle: blunt at the nose, thin towards the tail.
    let radius = |x: f32| -> f32 {
        let t = if x > 0.1 {
            (x - 0.1) / 0.42
        } else {
            (0.1 - x) / 0.62
        };
        (1.0 - t * t).max(0.0).sqrt() * 0.5
    };
    vertices.push([0.5, 0.0, 0.0, 0.0]); // the nose
    for ring in 0..RINGS {
        let x = 0.5 - (ring as f32 + 1.0) / (RINGS as f32 + 1.0);
        let r = radius(x).max(0.02);
        for s in 0..SEGMENTS {
            let a = s as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            vertices.push([x, r * a.cos(), r * a.sin(), 0.0]);
        }
    }
    let tail = vertices.len() as u16;
    vertices.push([-0.5, 0.0, 0.0, 0.0]);
    let ring_start = |ring: usize| 1 + (ring * SEGMENTS) as u16;
    for s in 0..SEGMENTS {
        let next = (s + 1) % SEGMENTS;
        indices.extend_from_slice(&[0, ring_start(0) + s as u16, ring_start(0) + next as u16]);
        for ring in 0..RINGS - 1 {
            let (a, b) = (ring_start(ring), ring_start(ring + 1));
            let (s, n) = (s as u16, next as u16);
            indices.extend_from_slice(&[a + s, b + s, a + n, a + n, b + s, b + n]);
        }
        let last = ring_start(RINGS - 1);
        indices.extend_from_slice(&[last + s as u16, tail, last + next as u16]);
    }
    // The forked tail fin, flat in the vertical plane.
    let fin = vertices.len() as u16;
    for v in [
        [-0.44, 0.0, 0.0],
        [-0.64, 0.42, 0.0],
        [-0.56, 0.0, 0.0],
        [-0.64, -0.42, 0.0],
    ] {
        vertices.push([v[0], v[1], v[2], 1.0]);
    }
    indices.extend_from_slice(&[fin, fin + 1, fin + 2, fin, fin + 2, fin + 3]);
    (vertices, indices)
}

pub(crate) struct Fish {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    instances: wgpu::Buffer,
    uniforms: wgpu::Buffer,
    group: wgpu::BindGroup,
    count: u32,
}

impl Fish {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let source = [
            include_str!("../shaders/common.wgsl"),
            include_str!("../shaders/fish.wgsl"),
        ]
        .concat();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fish"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let mesh_attributes = [wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 0,
            shader_location: 0,
        }];
        let fish_attributes = [1u32, 2, 3].map(|location| wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 16 * (location as u64 - 1),
            shader_location: location,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fish"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("fish_vertex"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 16,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &mesh_attributes,
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: FISH_STRIDE,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &fish_attributes,
                    }),
                ],
            },
            // The fin is seen from both sides.
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
                entry_point: Some("fish_fragment"),
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
        let buffer = |label, usage, size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size.next_multiple_of(4),
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let mesh_bytes: &[u8] = bytemuck::cast_slice(&mesh);
        let index_bytes: &[u8] = bytemuck::cast_slice(&index_data);
        let vertices = buffer(
            "fish vertices",
            wgpu::BufferUsages::VERTEX,
            mesh_bytes.len() as u64,
        );
        queue.write_buffer(&vertices, 0, mesh_bytes);
        let indices = buffer(
            "fish indices",
            wgpu::BufferUsages::INDEX,
            index_bytes.len() as u64,
        );
        queue.write_buffer(&indices, 0, &pad(index_bytes));
        let instances = buffer(
            "fish instances",
            wgpu::BufferUsages::VERTEX,
            MAX_FISH as u64 * FISH_STRIDE,
        );
        let uniforms = buffer("fish uniforms", wgpu::BufferUsages::UNIFORM, 4 * 16);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fish"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        Fish {
            pipeline,
            vertices,
            indices,
            index_count: index_data.len() as u32,
            instances,
            uniforms,
            group,
            count: 0,
        }
    }

    /// Uploads the frame's fish (at most `MAX_FISH`) and uniforms. Call before submitting the encoder that
    /// holds [`Fish::draw`]. `size` is the scene target's size in pixels; nothing is drawn when `visible`
    /// is false.
    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        frame: &SceneFrame,
        size: (u32, u32),
        visible: bool,
    ) {
        let fish = &frame.fish[..frame.fish.len().min(MAX_FISH)];
        self.count = if visible { fish.len() as u32 } else { 0 };
        if self.count == 0 {
            return;
        }
        let v = |a: [f64; 3], w: f64| [a[0] as f32, a[1] as f32, a[2] as f32, w as f32];
        let instances: Vec<[f32; 4]> = fish
            .iter()
            .flat_map(|f| {
                let kind = match f.kind {
                    FishKind::Baitfish => 0.0,
                    FishKind::Mackerel => 1.0,
                };
                [
                    v(f.position, f.size),
                    v(f.heading, f.speed),
                    [kind, f.tail_phase as f32, 0.0, 0.0],
                ]
            })
            .collect();
        queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&instances));

        let sky = &frame.sky;
        let uniforms = [
            [(size.0 as f64 / size.1 as f64) as f32, 0.0, 0.0, 0.0],
            [1.0 / size.1 as f32, sky.warmth as f32, 0.0, 0.0],
            v(sky.light_direction, sky.light_intensity),
            v(sky.light_colour, sky.ambient),
        ];
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&uniforms));
    }

    /// Draws the fish into the scene pass, after the kelp and the whales.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, self.instances.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.index_count, 0, 0..self.count);
    }
}

/// `bytes` padded with zeros to a multiple of four, as `write_buffer` requires.
fn pad(bytes: &[u8]) -> Vec<u8> {
    let mut padded = bytes.to_vec();
    padded.resize(bytes.len().next_multiple_of(4), 0);
    padded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_is_a_closed_body_with_a_tail_fin() {
        let (vertices, indices) = build_mesh();
        assert_eq!(vertices.len(), 2 + RINGS * SEGMENTS + 4);
        assert_eq!(
            indices.len(),
            SEGMENTS * 3 * 2 + (RINGS - 1) * SEGMENTS * 6 + 6
        );
        assert!(indices.iter().all(|&i| (i as usize) < vertices.len()));
        assert!(vertices.iter().all(|v| v[0] >= -0.65 && v[0] <= 0.5));
    }
}
