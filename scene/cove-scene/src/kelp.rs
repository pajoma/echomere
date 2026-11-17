//! Kelp motion. Port of `Kelp.swift`.

use crate::model::scene_rules;
use crate::sea;
use std::f64::consts::PI;

/// The kelp is a frame of long ribbon blades at the picture's left and right edges, moving like the
/// blades of abyssal-garden (`W_LEAF`). How fast the blades' own motion runs against real time (owner,
/// 2026-11-17: "even a bit slower", the same pace as the sea).
pub const KELP_PACE: f64 = 0.25;

/// One kelp blade, rooted below the picture near one of its side edges. Distances in metres: x is to the
/// right of the camera, y up with the surface at 0, z ahead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KelpBlade {
    pub root: [f64; 3],
    pub length: f64,
    /// The picture-x ratio (sideways offset over distance along the view axis, at 16:10) the blade's line
    /// heads for as it rises, so the whole blade stays near the edge where it is rooted.
    pub lean: f64,
    /// How far the blade leans away from (+) or towards (-) the camera, -0.2..0.2.
    pub forward: f64,
    /// Full width at the widest point.
    pub width: f64,
    /// How strongly the blade curls, 0..1 (`W_LEAF`'s per-blade curvature).
    pub curvature: f64,
    /// `W_LEAF`'s per-blade seeds s1 and s2, each 0..1.
    pub seed: [f64; 2],
    /// -1 at the left edge, +1 at the right.
    pub side: f64,
    /// Offsets this blade's swell push, in radians.
    pub sway_phase: f64,
}

/// Where the kelp is in its movement at one moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KelpMotion {
    /// The back-and-forth surge of the swell, as an angle 0..2π.
    pub surge: f64,
    /// The kelp clock in seconds: real time within the day, slowed by `KELP_PACE`. It repeats daily, like
    /// the water's shader time.
    pub time: f64,
}

/// The period of the swell over the cove's seabed, in real seconds: ω² = g·k·tanh(k·h), slowed with the
/// sea (`sea::SEA_PACE`) so the kelp keeps time with the waves above it.
pub fn surge_period() -> f64 {
    let k = 2.0 * PI / sea::SWELL_WAVELENGTH;
    let omega = (9.81 * k * (k * scene_rules::CAMERA_DEPTH).tanh()).sqrt();
    2.0 * PI / omega / sea::SEA_PACE
}

pub fn motion(at: f64) -> KelpMotion {
    KelpMotion {
        surge: cycle(at, surge_period()),
        time: at.rem_euclid(86_400.0) * KELP_PACE,
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
