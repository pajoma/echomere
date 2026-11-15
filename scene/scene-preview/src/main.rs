//! Shows the cove scene in a window, or writes snapshots and frame sequences. The Rust port of
//! `ScenePreview` (macOS).
//!
//! ```text
//! cargo run -p scene-preview -- [--seed N] [--whales N] [--echoes N] [--speed X] [--fps N] [--scale X]
//! cargo run -p scene-preview -- --snapshot out.png [--at UNIX_SECONDS] [--size 1600x1000] [--seed N] ...
//! cargo run -p scene-preview -- --frames DIR --seconds N [--fps N] [--at UNIX_SECONDS] [--size WxH] ...
//! ```
//!
//! Sky: `--hour H` shows that hour of the day; `--day-seconds N` runs a whole day in N seconds, starting at
//! `--hour` (the waves and the kelp keep their real speed). Numbered PNGs from `--frames` turn into a clip
//! with `ffmpeg -framerate 15 -i DIR/frame-%05d.png -pix_fmt yuv420p clip.mp4`.
//!
//! In the window: space shows another cove, Esc quits. `--stats` prints the frame rate and the GPU time
//! per frame once a second.

mod sample_cove;

use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::Parser;
use cove_render::Renderer;
use cove_scene::model::{scene_rules, CoveSeed, Echo, Visitor};
use cove_scene::scene::{CoveScene, SceneFrame};
use cove_scene::sky::{self, SkyState};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

#[derive(Parser, Debug, Clone)]
#[command(about = "Shows the Echomere cove scene, or writes snapshots of it.")]
struct Options {
    /// The cove's seed.
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Whales visiting (0 to the cove's maximum).
    #[arg(long, default_value_t = 3)]
    whales: usize,
    /// Echoes of whales that left (0 to 8).
    #[arg(long, default_value_t = 2)]
    echoes: usize,
    /// How fast scene time runs.
    #[arg(long, default_value_t = 1.0)]
    speed: f64,
    /// Frames per second (1 to 60).
    #[arg(long, default_value_t = 15)]
    fps: u32,
    /// The fraction of the output size the scene is drawn at (0.1 to 1).
    #[arg(long, default_value_t = 0.5)]
    scale: f64,
    /// `above` shows the sea from above, for checking the waves.
    #[arg(long)]
    view: Option<String>,
    /// The hour of the day to show, instead of the real one.
    #[arg(long)]
    hour: Option<f64>,
    /// Runs a whole day in this many seconds, starting at `--hour` (5 if not given).
    #[arg(long = "day-seconds")]
    day_seconds: Option<f64>,
    /// Writes one picture to this PNG file and quits.
    #[arg(long)]
    snapshot: Option<String>,
    /// Writes numbered PNG frames into this directory and quits.
    #[arg(long)]
    frames: Option<String>,
    /// How many seconds of frames `--frames` writes.
    #[arg(long, default_value_t = 10.0)]
    seconds: f64,
    /// The scene time, in seconds since 1970, instead of now.
    #[arg(long)]
    at: Option<f64>,
    /// The size of snapshots and frames, as WxH.
    #[arg(long, default_value = "1600x1000", value_parser = parse_size)]
    size: (u32, u32),
    /// Prints the frame rate and the GPU time per frame (from timestamp queries, when the adapter has them).
    #[arg(long)]
    stats: bool,
}

fn parse_size(value: &str) -> Result<(u32, u32), String> {
    let parts: Vec<u32> = value.split('x').filter_map(|p| p.parse().ok()).collect();
    match parts[..] {
        [w, h] if w > 0 && h > 0 => Ok((w, h)),
        _ => Err(format!("expected WxH, such as 1600x1000, not {value}")),
    }
}

impl Options {
    /// Clamps the values the way the macOS preview does.
    fn clamped(mut self) -> Self {
        self.whales = self.whales.min(scene_rules::MAX_WHALES);
        self.echoes = self.echoes.min(8);
        self.fps = self.fps.clamp(1, 60);
        self.scale = self.scale.clamp(0.1, 1.0);
        self
    }

    fn sea_from_above(&self) -> bool {
        self.view.as_deref() == Some("above")
    }

    /// The sky to show `elapsed` seconds after the start, or `None` for the real sky.
    fn sky(&self, elapsed: f64) -> Option<SkyState> {
        if self.hour.is_none() && self.day_seconds.is_none() {
            return None;
        }
        let start = self.hour.unwrap_or(5.0);
        Some(sky::state(start + self.day_seconds.map_or(0.0, |d| elapsed * 24.0 / d)))
    }

    /// The scene time at the start: `--at`, or now.
    fn start_time(&self) -> f64 {
        self.at.unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64()))
    }
}

/// A cove with its sample guests.
struct Cove {
    scene: CoveScene,
    visitors: Vec<Visitor>,
    echoes: Vec<Echo>,
}

impl Cove {
    /// The cove of `seed` as it is at `now`: the visitors arrived in the hours before (the last one 5 s
    /// after `visitors_now`), the echoes left in the days before.
    fn new(seed: u64, options: &Options, visitors_now: f64, now: f64) -> Self {
        Cove {
            scene: CoveScene::new(CoveSeed(seed)),
            visitors: sample_cove::visitors(seed, options.whales, visitors_now),
            echoes: sample_cove::echoes(seed, options.echoes, now),
        }
    }

    fn frame(&self, at: f64, sky: Option<SkyState>) -> SceneFrame {
        let mut frame = self.scene.frame(at, &self.visitors, &self.echoes);
        if let Some(sky) = sky {
            frame.sky = sky;
        }
        frame
    }
}

fn main() -> ExitCode {
    let options = Options::parse().clamped();
    let result = if let Some(directory) = options.frames.clone() {
        write_frames(&options, &directory)
    } else if let Some(path) = options.snapshot.clone() {
        write_snapshot(&options, &path)
    } else {
        run_window(options)
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn headless(options: &Options) -> Result<Renderer, String> {
    let mut renderer = Renderer::new_headless().map_err(|e| format!("Renderer failed: {e}"))?;
    renderer.set_scale(options.scale as f32);
    renderer.set_sea_from_above(options.sea_from_above());
    Ok(renderer)
}

fn write_snapshot(options: &Options, path: &str) -> Result<(), String> {
    let mut renderer = headless(options)?;
    let time = options.start_time();
    let cove = Cove::new(options.seed, options, time - 600.0, time);
    let frame = cove.frame(time, options.sky(0.0));
    let (width, height) = options.size;
    let image = renderer.snapshot(&frame, width, height).map_err(|e| format!("Snapshot failed: {e}"))?;
    image.save(path).map_err(|e| format!("Snapshot failed: {e}"))?;
    println!(
        "Wrote {path}: seed {}, {} whales, {} fish, {} kelp, {} echoes",
        options.seed,
        frame.whales.len(),
        frame.creatures.len(),
        frame.kelp.len(),
        frame.echoes.len()
    );
    Ok(())
}

fn write_frames(options: &Options, directory: &str) -> Result<(), String> {
    let mut renderer = headless(options)?;
    let start = options.start_time();
    let cove = Cove::new(options.seed, options, start - 600.0, start);
    let fps = f64::from(options.fps);
    let count = (options.seconds * fps) as usize;
    std::fs::create_dir_all(directory).map_err(|e| format!("Frames failed: {e}"))?;
    let (width, height) = options.size;
    for index in 0..count {
        let elapsed = index as f64 / fps;
        let frame = cove.frame(start + elapsed * options.speed, options.sky(elapsed));
        let image = renderer.snapshot(&frame, width, height).map_err(|e| format!("Frames failed: {e}"))?;
        let path = Path::new(directory).join(format!("frame-{index:05}.png"));
        image.save(&path).map_err(|e| format!("Frames failed: {e}"))?;
    }
    println!("Wrote {count} frames to {directory}");
    Ok(())
}

fn run_window(options: Options) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("No window system: {e}"))?;
    // A check without hands: SCENE_PREVIEW_SELF_TEST=1 resizes the window after 6 s and quits after 12 s.
    let self_test = std::env::var_os("SCENE_PREVIEW_SELF_TEST").map(|_| SelfTest { resized: false });
    let mut app = Preview { options, state: None, error: None, self_test };
    event_loop.run_app(&mut app).map_err(|e| format!("Window failed: {e}"))?;
    match app.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// The window and everything that draws into it.
struct Preview {
    options: Options,
    state: Option<WindowState>,
    error: Option<String>,
    self_test: Option<SelfTest>,
}

struct SelfTest {
    resized: bool,
}

struct WindowState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    seed: u64,
    cove: Cove,
    started: Instant,
    /// The scene time at `started`.
    scene_start: f64,
    /// When the next frame is due.
    next_frame: Instant,
    /// Nothing can see the window (minimised or covered): nothing is drawn.
    occluded: bool,
    stats: Option<Stats>,
}

impl WindowState {
    fn new(event_loop: &ActiveEventLoop, options: &Options) -> Result<Self, String> {
        let attributes = Window::default_attributes()
            .with_title(format!("Echomere: cove {}", options.seed))
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
        let window = Arc::new(event_loop.create_window(attributes).map_err(|e| format!("No window: {e}"))?);

        let display = event_loop.owned_display_handle();
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(display)));
        let surface = instance.create_surface(window.clone()).map_err(|e| format!("No surface: {e}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|e| format!("No graphics adapter: {e}"))?;
        // Timestamps only when asked for, and only if the adapter has them.
        let timestamps = options.stats && adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("cove preview"),
            required_features: if timestamps { wgpu::Features::TIMESTAMP_QUERY } else { wgpu::Features::empty() },
            ..Default::default()
        }))
        .map_err(|e| format!("Cannot open the graphics device: {e}"))?;

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("The surface does not work with this adapter")?;
        config.format = output_format(&surface.get_capabilities(&adapter).formats)
            .ok_or("The surface has no format the scene can draw into")?;
        config.view_formats.clear();
        surface.configure(&device, &config);

        let stats = options.stats.then(|| Stats::new(&device, &queue, timestamps));
        if options.stats {
            println!(
                "Adapter: {} ({:?}), surface {:?} {}x{}, GPU timestamps: {}",
                adapter.get_info().name,
                adapter.get_info().backend,
                config.format,
                config.width,
                config.height,
                if timestamps { "yes" } else { "no (CPU time only)" }
            );
        }
        let mut renderer = Renderer::new(instance, adapter, device, queue, config.format)
            .map_err(|e| format!("Renderer failed: {e}"))?;
        renderer.set_scale(options.scale as f32);
        renderer.set_sea_from_above(options.sea_from_above());

        let scene_start = options.start_time();
        let started = Instant::now();
        let cove = Cove::new(options.seed, options, scene_start, scene_start);
        Ok(WindowState {
            window,
            surface,
            config,
            renderer,
            seed: options.seed,
            cove,
            started,
            scene_start,
            next_frame: started,
            occluded: false,
            stats,
        })
    }

    fn elapsed(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    fn now(&self, options: &Options) -> f64 {
        self.scene_start + self.elapsed() * options.speed
    }

    /// Shows the cove of `seed`, with guests as they are now.
    fn load_cove(&mut self, seed: u64, options: &Options) {
        self.seed = seed;
        let now = self.now(options);
        self.cove = Cove::new(seed, options, now, now);
        self.window.set_title(&format!("Echomere: cove {seed}"));
    }

    /// Reconfigures the surface for the window's new size. The renderer rebuilds its targets on the next
    /// frame, when it sees the new size.
    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(self.renderer.device(), &self.config);
    }

    fn draw(&mut self, options: &Options) {
        if self.occluded || self.config.width == 0 || self.config.height == 0 {
            return;
        }
        let texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(self.renderer.device(), &self.config);
                return;
            }
            // Skip this frame and try again at the next.
            _ => return,
        };
        let view = texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let frame = self.cove.frame(self.now(options), options.sky(self.elapsed()));
        let cpu = Instant::now();
        if let Some(stats) = &self.stats {
            stats.begin(self.renderer.device(), self.renderer.queue());
        }
        self.renderer.render(&frame, &view, (self.config.width, self.config.height));
        if let Some(stats) = &mut self.stats {
            stats.end(self.renderer.device(), self.renderer.queue(), cpu);
        }
        self.window.pre_present_notify();
        self.renderer.queue().present(texture);
    }
}

/// The non-sRGB variant of the surface's preferred format (`finish()` gamma-encodes itself), or the first
/// non-sRGB format it offers.
fn output_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    let preferred = formats.first()?.remove_srgb_suffix();
    if formats.contains(&preferred) {
        Some(preferred)
    } else {
        formats.iter().copied().find(|f| !f.is_srgb())
    }
}

impl ApplicationHandler for Preview {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        match WindowState::new(event_loop, &self.options) {
            Ok(state) => {
                state.window.request_redraw();
                self.state = Some(state);
            }
            Err(error) => {
                self.error = Some(error);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = &mut self.state else { return };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if self.options.stats {
                    println!("Resized to {}x{}", size.width, size.height);
                }
                state.resize(size.width, size.height);
            }
            WindowEvent::Occluded(occluded) => {
                if self.options.stats {
                    println!("Occluded: {occluded}");
                }
                state.occluded = occluded;
                if !occluded {
                    state.next_frame = Instant::now();
                    state.window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent { logical_key, state: ElementState::Pressed, repeat: false, .. },
                ..
            } => match logical_key {
                Key::Named(NamedKey::Space) => state.load_cove(state.seed.wrapping_add(1), &self.options),
                Key::Named(NamedKey::Escape) => event_loop.exit(),
                _ => {}
            },
            WindowEvent::RedrawRequested => state.draw(&self.options),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = &mut self.state else { return };
        if let Some(test) = &mut self.self_test {
            let elapsed = state.elapsed();
            if elapsed >= 6.0 && !test.resized {
                test.resized = true;
                println!("Self-test: asking for 800x500");
                let _ = state.window.request_inner_size(winit::dpi::PhysicalSize::new(800, 500));
            }
            if elapsed >= 12.0 {
                println!("Self-test: quitting");
                event_loop.exit();
                return;
            }
        }
        if state.occluded {
            // Wait for the window to be seen again; no timer, no frames.
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        let now = Instant::now();
        if now >= state.next_frame {
            state.window.request_redraw();
            let interval = Duration::from_secs_f64(1.0 / f64::from(self.options.fps));
            // Keep the beat, but don't try to catch up frames that were missed.
            state.next_frame = (state.next_frame + interval).max(now);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(state.next_frame));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(stats) = self.state.as_ref().and_then(|s| s.stats.as_ref()) {
            println!("Drew {} frames in {:.1} s", stats.total_frames, stats.since_start.elapsed().as_secs_f64());
        }
    }
}

/// Frame rate and GPU time per frame, printed once a second. The GPU time comes from a timestamp written
/// before the frame's work and one after it.
struct Stats {
    timestamps: Option<Timestamps>,
    since_start: Instant,
    window_start: Instant,
    frames: u32,
    total_frames: u64,
    gpu_ms: f64,
    cpu_ms: f64,
}

struct Timestamps {
    queries: wgpu::QuerySet,
    resolved: wgpu::Buffer,
    readback: wgpu::Buffer,
    period_ns: f64,
}

impl Stats {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, timestamps: bool) -> Self {
        let timestamps = timestamps.then(|| Timestamps {
            queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("frame time"),
                ty: wgpu::QueryType::Timestamp,
                count: 2,
            }),
            resolved: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frame time resolved"),
                size: 16,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            readback: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frame time readback"),
                size: 16,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            period_ns: f64::from(queue.get_timestamp_period()),
        });
        let now = Instant::now();
        Stats { timestamps, since_start: now, window_start: now, frames: 0, total_frames: 0, gpu_ms: 0.0, cpu_ms: 0.0 }
    }

    /// An empty pass that writes a timestamp, submitted before the frame.
    fn begin(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if let Some(t) = &self.timestamps {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("begin") });
            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("frame begins"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &t.queries,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: None,
                }),
            });
            queue.submit([encoder.finish()]);
        }
    }

    /// The closing timestamp; waits for the frame to finish and adds it up.
    fn end(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, cpu_start: Instant) {
        if let Some(t) = &self.timestamps {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("end") });
            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("frame ends"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &t.queries,
                    beginning_of_pass_write_index: None,
                    end_of_pass_write_index: Some(1),
                }),
            });
            encoder.resolve_query_set(&t.queries, 0..2, &t.resolved, 0);
            encoder.copy_buffer_to_buffer(&t.resolved, 0, &t.readback, 0, 16);
            queue.submit([encoder.finish()]);
            t.readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        }
        // Waiting here makes the CPU time cover the GPU's work too.
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        self.cpu_ms += cpu_start.elapsed().as_secs_f64() * 1000.0;
        if let Some(t) = &self.timestamps {
            if let Ok(range) = t.readback.slice(..).get_mapped_range() {
                let tick = |i: usize| u64::from_le_bytes(range[i * 8..i * 8 + 8].try_into().expect("8 bytes"));
                let ticks = [tick(0), tick(1)];
                self.gpu_ms += ticks[1].wrapping_sub(ticks[0]) as f64 * t.period_ns / 1e6;
            }
            t.readback.unmap();
        }
        self.frames += 1;
        self.total_frames += 1;
        let seconds = self.window_start.elapsed().as_secs_f64();
        if seconds >= 1.0 {
            let n = f64::from(self.frames);
            let gpu = if self.timestamps.is_some() { format!("{:.1} ms", self.gpu_ms / n) } else { "n/a".into() };
            println!(
                "{:.1} fps, GPU {gpu} per frame, CPU+wait {:.1} ms per frame, {} frames so far",
                n / seconds,
                self.cpu_ms / n,
                self.total_frames
            );
            self.window_start = Instant::now();
            self.frames = 0;
            self.gpu_ms = 0.0;
            self.cpu_ms = 0.0;
        }
    }
}
