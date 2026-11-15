//! The composite pass: bilinear upscale of the scene target to the output, and `finish()` once per
//! output pixel. The shaders are `shaders/common.wgsl` and `shaders/composite.wgsl`.

use crate::targets::Targets;

pub(crate) struct Composite {
    module: wgpu::ShaderModule,
    sampler: wgpu::Sampler,
    /// One pipeline per output format (the window's surface, and `Rgba8Unorm` for snapshots).
    pipelines: Vec<(wgpu::TextureFormat, wgpu::RenderPipeline)>,
}

impl Composite {
    pub fn new(device: &wgpu::Device) -> Self {
        let source = [include_str!("../shaders/common.wgsl"), include_str!("../shaders/composite.wgsl")].concat();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("composite"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bilinear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Composite { module, sampler, pipelines: Vec::new() }
    }

    fn pipeline(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) -> &wgpu::RenderPipeline {
        if let Some(i) = self.pipelines.iter().position(|(f, _)| *f == format) {
            return &self.pipelines[i].1;
        }
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("composite"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &self.module,
                entry_point: Some("composite_vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &self.module,
                entry_point: Some("composite_fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        self.pipelines.push((format, pipeline));
        &self.pipelines.last().expect("just pushed").1
    }

    /// Draws the scene target into `output`, which has the format `format`.
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        targets: &Targets,
        output: &wgpu::TextureView,
        format: wgpu::TextureFormat,
    ) {
        let pipeline = self.pipeline(device, format).clone();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&targets.color) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
}
