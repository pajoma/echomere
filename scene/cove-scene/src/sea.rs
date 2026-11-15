//! The sea surface above every cove. Port of `Sea.swift`.
//!
//! An FFT ocean (Tessendorf) with a wind sea and a long swell, simulated on the GPU. It is the same
//! for every cove and every screen, and repeats seamlessly every `LOOP_PERIOD` seconds.

/// Wind speed 10 m above the sea, in m/s.
pub const WIND_SPEED: f64 = 11.0;
/// Direction the wind waves travel, in degrees from the x axis (90 is away from the camera).
pub const WIND_DIRECTION: f64 = 118.0;
/// Distance the wind has blown over open water, in metres.
pub const FETCH: f64 = 90_000.0;
pub const JONSWAP_GAMMA: f64 = 3.3;
/// Significant height of the swell, in metres.
pub const SWELL_HEIGHT: f64 = 2.4;
pub const SWELL_DIRECTION: f64 = 94.0;
pub const SWELL_WAVELENGTH: f64 = 120.0;
/// How narrow the swell's directions are (cos^2s exponent): larger is more long-crested.
pub const SWELL_SPREAD: f64 = 48.0;
/// How much the crests are sharpened by horizontal displacement.
pub const CHOPPINESS: f64 = 1.45;
/// Waves shorter than about this length (m) are damped.
pub const SHORT_WAVE_CUT: f64 = 0.0012;
/// Patch sizes of the simulation cascades, in metres: long swell, wind sea, ripples.
pub const CASCADE_SIZES: [f64; 3] = [211.3, 39.7, 7.43];
/// The sea repeats after this many seconds.
pub const LOOP_PERIOD: f64 = 600.0;
/// The seed for the random phases of the spectrum.
pub const SEED: u32 = 1;

/// The time within the loop, in seconds, computed in double precision.
pub fn time(at: f64) -> f64 {
    let t = at % LOOP_PERIOD;
    if t < 0.0 {
        t + LOOP_PERIOD
    } else {
        t
    }
}
