// `max(..).min(..)` is kept over `clamp` on purpose: it matches the Swift original and swallows NaN instead of returning it.
#![allow(clippy::manual_clamp)]
pub mod camera;
pub mod kelp;
pub mod layout;
pub mod model;
pub mod random;
pub mod scene;
pub mod schools;
pub mod sea;
pub mod sky;

pub use random::{combine, stable_hash, SplitMix64};
