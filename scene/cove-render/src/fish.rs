//! The fish pass: one full-screen triangle, last in the scene pass, that draws every small fish from the
//! scene buffer over everything else. It writes linear light premultiplied by the fish's coverage. Port of
//! the fish draw of `CoveSceneRenderer.swift`; the shaders are `shaders/common.wgsl`,
//! `shaders/scene_buffer.wgsl` and `shaders/fish.wgsl`.

use cove_scene::scene::SceneFrame;

use crate::targets::{COLOR_FORMAT, DEPTH_FORMAT};

pub(crate) struct Fish {
    pipeline: wgpu::RenderPipeline,
    group: wgpu::BindGroup,
    visible: bool,
}

impl Fish {
    /// `scene_buffer` is the water pass's scene buffer ([`crate::water::SceneBuffer`]), which holds the fish.
    pub fn new(device: &wgpu::Device, scene_buffer: &wgpu::Buffer) -> Self {
        let source = [
            include_str!("../shaders/common.wgsl"),
            include_str!("../shaders/scene_buffer.wgsl"),
            include_str!("../shaders/fish.wgsl"),
        ]
        .concat();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fish"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fish"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("fullscreen_vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            // The fish swim near the camera, in front of everything: no depth test, no depth write.
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
                entry_point: Some("fish_fragment"),
                compilation_options: Default::default(),
                // The colour is premultiplied by the coverage: (one, one - alpha), in linear light.
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fish"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: scene_buffer.as_entire_binding() }],
        });
        Fish { pipeline, group, visible: false }
    }

    /// Decides whether the fish are drawn this frame. The scene buffer is uploaded by the water pass.
    pub fn prepare(&mut self, frame: &SceneFrame, visible: bool) {
        self.visible = visible && !frame.creatures.is_empty();
    }

    /// Draws the fish into the scene pass, last.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if !self.visible {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.draw(0..3, 0..1);
    }
}
