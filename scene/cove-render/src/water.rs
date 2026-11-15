//! The water pass: a full-screen triangle drawing the open water, the underside of the surface, echoes,
//! marine snow and the light shafts into the scene target, in linear light. The shaders are
//! `shaders/common.wgsl` and `shaders/water.wgsl`; the uniforms are packed by [`SceneBuffer`].

use cove_scene::scene::SceneFrame;

use crate::ocean::Ocean;
use crate::targets::{COLOR_FORMAT, DEPTH_FORMAT};

/// Packs a scene frame into the flat list of `vec4f` the water shader reads. Port of `SceneBuffer.swift`;
/// the layout must match the constants in `scene_buffer.wgsl`.
pub(crate) struct SceneBuffer;

impl SceneBuffer {
    /// The sky: light direction and intensity, light colour and ambient, zenith, horizon.
    pub const SKY_START: usize = 2;
    /// The small fish, three `vec4f` each. Packed for the fish pass; the water pass does not draw them.
    pub const CREATURE_START: usize = 14;
    pub const ECHO_START: usize = 326;
    pub const COUNT: usize = 336;
    pub const MAX_CREATURES: usize = (Self::ECHO_START - Self::CREATURE_START) / 3;
    pub const MAX_ECHOES: usize = (Self::COUNT - Self::ECHO_START) / 2;

    /// `aspect` is width / height of the scene target, `pixels_high` its height in pixels (for
    /// anti-aliasing). `sea_from_above` switches to the debug view of the sea from above.
    pub fn pack(frame: &SceneFrame, aspect: f64, pixels_high: f64, sea_from_above: bool) -> Vec<[f32; 4]> {
        let mut data = vec![[0.0f32; 4]; Self::COUNT];
        let creatures = &frame.creatures[..frame.creatures.len().min(Self::MAX_CREATURES)];
        let echoes = &frame.echoes[..frame.echoes.len().min(Self::MAX_ECHOES)];

        // Shader time runs over one day; f32 keeps enough precision for slow motion at that range.
        let shader_time = frame.time % 86_400.0;
        data[0] = [aspect as f32, shader_time as f32, creatures.len() as f32, 0.0];
        data[1] = [echoes.len() as f32, (1.0 / pixels_high) as f32, if sea_from_above { 1.0 } else { 0.0 }, 0.0];

        for (i, c) in creatures.iter().enumerate() {
            let b = Self::CREATURE_START + i * 3;
            data[b] = [c.x as f32, c.y as f32, c.depth as f32, c.facing as f32];
            data[b + 1] = [c.phase as f32, c.size as f32, 1.0, 0.0];
            data[b + 2] = [c.colour as f32, 1.0, 0.0, 0.0];
        }

        let sky = &frame.sky;
        let v = |a: [f64; 3], w: f64| [a[0] as f32, a[1] as f32, a[2] as f32, w as f32];
        data[Self::SKY_START] = v(sky.light_direction, sky.light_intensity);
        data[Self::SKY_START + 1] = v(sky.light_colour, sky.ambient);
        data[Self::SKY_START + 2] = v(sky.zenith, 0.0);
        data[Self::SKY_START + 3] = v(sky.horizon, 0.0);

        for (i, e) in echoes.iter().enumerate() {
            let b = Self::ECHO_START + i * 2;
            data[b] = [e.x as f32, e.y as f32, e.depth as f32, e.intensity as f32];
            data[b + 1] = [e.call.unwrap_or(-1.0) as f32, e.size as f32, 0.0, 0.0];
        }
        data
    }
}

pub(crate) struct Water {
    pipeline: wgpu::RenderPipeline,
    /// The scene buffer. The fish pass reads it too.
    pub uniforms: wgpu::Buffer,
    group: wgpu::BindGroup,
}

impl Water {
    pub fn new(device: &wgpu::Device, ocean: &Ocean) -> Self {
        let source = [
            include_str!("../shaders/common.wgsl"),
            include_str!("../shaders/scene_buffer.wgsl"),
            include_str!("../shaders/caustics.wgsl"),
            include_str!("../shaders/water.wgsl"),
        ]
        .concat();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("water"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("water"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("fullscreen_vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            // The backdrop: drawn first, behind everything, without touching the depth the geometry needs.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("water_fragment"),
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
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene buffer"),
            size: (SceneBuffer::COUNT * 16) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sea"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let array = |t: &wgpu::Texture| {
            t.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            })
        };
        let (displacement, derivatives, curvature) =
            (array(&ocean.displacement), array(&ocean.derivatives), array(&ocean.curvature));
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&displacement) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&derivatives) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&curvature) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        Water { pipeline, uniforms, group }
    }

    /// Uploads the frame's uniforms. Call before submitting the encoder that holds [`Water::draw`].
    pub fn prepare(&self, queue: &wgpu::Queue, frame: &SceneFrame, size: (u32, u32), sea_from_above: bool) {
        let data = SceneBuffer::pack(frame, size.0 as f64 / size.1 as f64, size.1 as f64, sea_from_above);
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&data));
    }

    /// Draws the water into the scene pass.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cove_scene::model::CoveSeed;
    use cove_scene::scene::CoveScene;

    #[test]
    fn scene_buffer_packs_like_swift() {
        let mut frame = CoveScene::new(CoveSeed(4_711)).frame(1_794_000_000.0 + 400.0 * 86_400.0, &[], &[]);
        frame.sky = cove_scene::sky::state(12.0);
        let data = SceneBuffer::pack(&frame, 1.6, 200.0, true);
        assert_eq!(data.len(), 336);
        assert_eq!(data[0][1], (1_794_000_000.0f64 % 86_400.0) as f32);
        assert_eq!(data[1][1..3], [1.0 / 200.0, 1.0]);
        assert_eq!(data[SceneBuffer::SKY_START + 1][3], frame.sky.ambient as f32);
        assert_eq!(SceneBuffer::MAX_CREATURES, 104);
        assert_eq!(SceneBuffer::MAX_ECHOES, 5);
    }
}
