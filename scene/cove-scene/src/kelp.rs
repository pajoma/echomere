//! Kelp motion. Port of `Kelp.swift`.

use crate::model::scene_rules;
use crate::sea;
use std::f64::consts::PI;

/// One stalk (stipe) of a giant kelp plant, in metres. x is to the right of the camera, z is ahead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KelpFrond {
    pub base_x: f64,
    pub base_z: f64,
    /// How far below the surface the stalk is rooted.
    pub base_depth: f64,
    /// Length of the stalk: its depth plus the part that trails along the surface.
    pub length: f64,
    /// Where this stalk is in its sway, relative to the swell (radians).
    pub sway_phase: f64,
    /// How the stalks of one plant wind around each other (radians).
    pub twist_phase: f64,
    /// Varies the blades from stalk to stalk.
    pub blade_seed: f64,
    pub blade_scale: f64,
}

/// Where the kelp is in its movement at one moment, as angles 0..2π.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KelpMotion {
    /// The back-and-forth surge of the swell.
    pub surge: f64,
    /// A slow sideways drift.
    pub drift: f64,
    /// The flutter of the blades.
    pub flutter: f64,
}

pub const DRIFT_PERIOD: f64 = 60.0;
pub const FLUTTER_PERIOD: f64 = 1.6;

/// The period of the swell over the cove's seabed, in seconds: ω² = g·k·tanh(k·h).
pub fn surge_period() -> f64 {
    let k = 2.0 * PI / sea::SWELL_WAVELENGTH;
    let omega = (9.81 * k * (k * scene_rules::CAMERA_DEPTH).tanh()).sqrt();
    2.0 * PI / omega
}

pub fn motion(at: f64) -> KelpMotion {
    KelpMotion {
        surge: cycle(at, surge_period()),
        drift: cycle(at, DRIFT_PERIOD),
        flutter: cycle(at, FLUTTER_PERIOD),
    }
}

/// Angle in 0..2π (negative for negative times, as in Swift) of time `t` within `period`.
pub fn cycle(t: f64, period: f64) -> f64 {
    (t / period) % 1.0 * 2.0 * PI
}

/// Wraps an angle into 0..2π.
pub fn wrap_angle(angle: f64) -> f64 {
    let wrapped = angle % (2.0 * PI);
    if wrapped < 0.0 {
        wrapped + 2.0 * PI
    } else {
        wrapped
    }
}
