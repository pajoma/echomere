//! The offscreen target the scene is drawn into: linear `rgba16float` colour plus `depth32float`, at a
//! fraction of the output size (half by default, to save energy).

/// The format of the scene's colour target. Light is linear in it, beyond 1 where it is bright.
pub(crate) const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The scene is drawn at this fraction of the output size by default.
pub(crate) const DEFAULT_SCALE: f32 = 0.5;

pub(crate) struct Targets {
    /// Size of the scene targets, in pixels.
    pub size: (u32, u32),
    pub color: wgpu::TextureView,
    pub depth: wgpu::TextureView,
}

impl Targets {
    /// The size of the scene targets for an output of `output` pixels.
    pub fn scaled_size(output: (u32, u32), scale: f32) -> (u32, u32) {
        let side = |v: u32| ((v as f32 * scale).round() as u32).max(1);
        (side(output.0), side(output.1))
    }

    pub fn new(device: &wgpu::Device, size: (u32, u32)) -> Self {
        let texture = |label, format, usage| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        Targets {
            size,
            color: texture("scene colour", COLOR_FORMAT, usage),
            depth: texture("scene depth", DEPTH_FORMAT, usage),
        }
    }
}
