//! The device, and the renderer that owns every pass.

use std::fmt;

use cove_scene::scene::SceneFrame;

use crate::composite::Composite;
use crate::kelp::Kelp;
use crate::ocean::{self, Ocean, SeaStatistics, CASCADES, N};
use crate::targets::{Targets, DEFAULT_SCALE};
use crate::water::Water;
use crate::whales::{self, Whales};

/// The format snapshots are read back in. Not sRGB, because `finish()` gamma-encodes itself.
const SNAPSHOT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[derive(Debug)]
pub enum RenderError {
    /// No graphics adapter at all, not even a software one.
    NoAdapter(String),
    /// The adapter could not open a device.
    NoDevice(String),
    /// Reading a result back from the GPU failed.
    Readback(String),
    /// A file the scene needs (such as a whale model) is missing or broken.
    Asset(String),
    /// The output format is sRGB. `finish()` gamma-encodes itself, so an sRGB output would encode twice.
    SrgbOutput(wgpu::TextureFormat),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::NoAdapter(e) => write!(f, "no graphics adapter: {e}"),
            RenderError::NoDevice(e) => write!(f, "cannot open the graphics device: {e}"),
            RenderError::Readback(e) => write!(f, "cannot read back from the GPU: {e}"),
            RenderError::Asset(e) => write!(f, "cannot load {e}"),
            RenderError::SrgbOutput(format) => {
                write!(f, "the output format {format:?} is sRGB; use its non-sRGB variant, the scene encodes itself")
            }
        }
    }
}

impl std::error::Error for RenderError {}

/// Draws `SceneFrame`s. Every frame simulates the sea, draws the scene into the half-resolution target
/// and composites it into the output.
pub struct Renderer {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// The format of the window's surface. It is never an sRGB format: `finish()` encodes itself.
    surface_format: wgpu::TextureFormat,
    scale: f32,
    targets: Option<Targets>,
    ocean: Ocean,
    water: Water,
    kelp: Kelp,
    whales: Whales,
    composite: Composite,
    /// Shows the sea from 4 m above instead of the cove, for checking the waves.
    sea_from_above: bool,
}

impl Renderer {
    /// A renderer without a window, for snapshots and checks. It takes a hardware adapter if there is
    /// one, and otherwise a software one (lavapipe on Linux).
    pub fn new_headless() -> Result<Self, RenderError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let request = |fallback| {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: fallback,
                ..Default::default()
            }))
        };
        let adapter = request(false)
            .or_else(|_| request(true))
            .map_err(|e| RenderError::NoAdapter(e.to_string()))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("cove"),
            ..Default::default()
        }))
        .map_err(|e| RenderError::NoDevice(e.to_string()))?;
        Self::new(instance, adapter, device, queue, SNAPSHOT_FORMAT)
    }

    /// A renderer for a device the caller opened, drawing into outputs of `surface_format`. It loads the
    /// whale models from the assets (see [`whales::assets_dir`]) and fails if they cannot be read.
    /// `surface_format` must not be an sRGB format, because `finish()` gamma-encodes itself.
    pub fn new(
        instance: wgpu::Instance,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Result<Self, RenderError> {
        if surface_format.is_srgb() {
            return Err(RenderError::SrgbOutput(surface_format));
        }
        let ocean = Ocean::new(&device, &queue);
        let water = Water::new(&device, &ocean);
        let kelp = Kelp::new(&device, &queue);
        let whales = Whales::new(&device, &queue, &ocean, &whales::whales_dir())?;
        let composite = Composite::new(&device);
        Ok(Renderer {
            instance,
            adapter,
            device,
            queue,
            surface_format,
            scale: DEFAULT_SCALE,
            targets: None,
            ocean,
            water,
            kelp,
            whales,
            composite,
            sea_from_above: false,
        })
    }

    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Draws the scene at this fraction of the output size (0.5 by default).
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale.clamp(0.05, 1.0);
    }

    /// Shows the sea from 4 m above, looking towards the sun, instead of the cove (a debug view).
    pub fn set_sea_from_above(&mut self, on: bool) {
        self.sea_from_above = on;
    }

    /// Draws `frame` into `target`, an output of `size` pixels in the surface format.
    pub fn render(&mut self, frame: &SceneFrame, target: &wgpu::TextureView, size: (u32, u32)) {
        self.draw(frame, target, size, self.surface_format);
    }

    fn draw(
        &mut self,
        frame: &SceneFrame,
        output: &wgpu::TextureView,
        size: (u32, u32),
        format: wgpu::TextureFormat,
    ) {
        let scene_size = Targets::scaled_size(size, self.scale);
        if self.targets.as_ref().map(|t| t.size) != Some(scene_size) {
            self.targets = Some(Targets::new(&self.device, scene_size));
        }
        let targets = self.targets.as_ref().expect("targets were just made");

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        self.ocean
            .simulate(&self.queue, &mut encoder, frame.sea_time);
        self.water
            .prepare(&self.queue, frame, scene_size, self.sea_from_above);
        // The debug view of the sea from above has no kelp or whales.
        self.kelp.prepare(
            &self.device,
            &self.queue,
            frame,
            scene_size,
            !self.sea_from_above,
        );
        self.whales.prepare(
            &self.device,
            &self.queue,
            frame,
            scene_size,
            !self.sea_from_above,
        );
        {
            let mut scene = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            self.water.draw(&mut scene);
            self.kelp.draw(&mut scene);
            self.whales.draw(&mut scene);
        }
        self.composite
            .draw(&self.device, &mut encoder, targets, output, format);
        self.queue.submit([encoder.finish()]);
    }

    /// Draws `frame` at `width` x `height` and reads the picture back.
    pub fn snapshot(
        &mut self,
        frame: &SceneFrame,
        width: u32,
        height: u32,
    ) -> Result<image::RgbaImage, RenderError> {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("snapshot"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SNAPSHOT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.draw(frame, &view, (width, height), SNAPSHOT_FORMAT);

        let row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let bytes = self.read_back(&texture, 0, row, height, 1)?;
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height as usize {
            let start = y * row as usize;
            pixels.extend_from_slice(&bytes[start..start + width as usize * 4]);
        }
        image::RgbaImage::from_raw(width, height, pixels)
            .ok_or_else(|| RenderError::Readback("snapshot has the wrong size".into()))
    }

    /// Simulates the sea at `sea_time` and measures it, for checks and tuning.
    pub fn sea_statistics(&mut self, sea_time: f64) -> Result<Vec<SeaStatistics>, RenderError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("sea") });
        self.ocean.simulate(&self.queue, &mut encoder, sea_time);
        self.queue.submit([encoder.finish()]);

        let halves = |texture: &wgpu::Texture| -> Result<Vec<u16>, RenderError> {
            let bytes = self.read_back(texture, 0, N * 8, N, CASCADES)?;
            Ok(bytes
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect())
        };
        let displacement = halves(&self.ocean.displacement)?;
        let derivatives = halves(&self.ocean.derivatives)?;
        let curvature = halves(&self.ocean.curvature)?;
        Ok((0..CASCADES as usize)
            .map(|c| SeaStatistics {
                cascade: c,
                height_rms: ocean::rms(&displacement, c, &[1]),
                slope_rms: ocean::rms(&derivatives, c, &[0, 1]),
                curvature_rms: ocean::rms(&curvature, c, &[0, 2]),
            })
            .collect())
    }

    /// Copies mip level `mip` of every layer of `texture` into memory, `bytes_per_row` per row.
    fn read_back(
        &self,
        texture: &wgpu::Texture,
        mip: u32,
        bytes_per_row: u32,
        rows: u32,
        layers: u32,
    ) -> Result<Vec<u8>, RenderError> {
        let size = (bytes_per_row * rows * layers) as u64;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback"),
            });
        let extent = texture.size().mip_level_size(mip, texture.dimension());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: mip,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(rows),
                },
            },
            wgpu::Extent3d {
                width: extent.width,
                height: rows,
                depth_or_array_layers: layers,
            },
        );
        self.queue.submit([encoder.finish()]);

        let (sender, receiver) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| RenderError::Readback(e.to_string()))?;
        receiver
            .recv()
            .map_err(|e| RenderError::Readback(e.to_string()))?
            .map_err(|e| RenderError::Readback(e.to_string()))?;
        let bytes = buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|e| RenderError::Readback(format!("{e:?}")))?
            .to_vec();
        buffer.unmap();
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_target_is_half_the_output_by_default() {
        assert_eq!(Targets::scaled_size((1440, 900), DEFAULT_SCALE), (720, 450));
        assert_eq!(Targets::scaled_size((1, 1), DEFAULT_SCALE), (1, 1));
    }
}
