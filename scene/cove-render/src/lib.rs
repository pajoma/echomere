//! The cove scene's renderer: wgpu with WGSL shaders, on macOS (Metal) and Linux (Vulkan, including the
//! software adapter lavapipe).
//!
//! Every frame first simulates the sea (an FFT ocean, see [`ocean`]), then draws the scene into a
//! half-resolution, linear `rgba16float` target, and finally a composite pass upscales it to the output
//! and applies `finish()` (tone map, vignette, gamma encode, dither) exactly once per output pixel.
// `chunks_exact` is kept over `as_chunks` so the workspace needs no newer Rust than it does now.
#![allow(clippy::chunks_exact_to_as_chunks)]

mod composite;
mod fish;
mod gpu;
mod kelp;
mod ocean;
mod targets;
mod water;
mod whales;

pub use gpu::{RenderError, Renderer};
pub use ocean::SeaStatistics;
pub use whales::{assets_dir, WhaleModel};
