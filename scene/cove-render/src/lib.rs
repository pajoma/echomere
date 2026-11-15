//! The cove scene's renderer: wgpu with WGSL shaders, on macOS (Metal) and Linux (Vulkan, including the
//! software adapter lavapipe).
//!
//! Every frame first simulates the sea (an FFT ocean, see [`ocean`]), then draws the scene into a
//! half-resolution, linear `rgba16float` target, and finally a composite pass upscales it to the output
//! and applies `finish()` (tone map, vignette, gamma encode, dither) exactly once per output pixel.

mod composite;
mod gpu;
mod kelp;
mod ocean;
mod targets;
mod water;

pub use gpu::{RenderError, Renderer};
pub use ocean::SeaStatistics;
