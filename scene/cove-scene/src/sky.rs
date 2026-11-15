//! The sky above the cove. Port of `Sky.swift`.

use std::f64::consts::PI;

type V3 = [f64; 3];

/// The sky above the cove at one moment. The sun is up from 6 to 18 o'clock and the moon from 18 to 6.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyState {
    /// The hour of the day, 0..24.
    pub hour: f64,
    /// Direction towards the sun by day, towards the moon by night (x right, y up, z ahead).
    pub light_direction: V3,
    /// 1 for the sun high in the sky, about 0.5 for the full moon, 0 while neither is up.
    pub light_intensity: f64,
    pub light_colour: V3,
    pub zenith: V3,
    pub horizon: V3,
    /// How bright the water itself is: 1 at the golden hour, more at noon, little at night.
    pub ambient: f64,
    pub is_night: bool,
}

pub const SUN_PEAK_ELEVATION: f64 = 60.0;
pub const MOON_PEAK_ELEVATION: f64 = 45.0;

/// The hour of the day at a moment (seconds since 1970). UTC for every cove.
pub fn hour(at: f64) -> f64 {
    let h = (at / 3600.0) % 24.0;
    if h < 0.0 {
        h + 24.0
    } else {
        h
    }
}

pub fn state_at(at: f64) -> SkyState {
    state(hour(at))
}

/// Elevation of the sun in degrees: 0 at 6 and 18 o'clock, highest at noon.
pub fn sun_elevation(hour: f64) -> f64 {
    SUN_PEAK_ELEVATION * (PI * (hour - 6.0) / 12.0).sin()
}

pub fn moon_elevation(hour: f64) -> f64 {
    MOON_PEAK_ELEVATION * (PI * (hour - 18.0) / 12.0).sin()
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub fn state(raw_hour: f64) -> SkyState {
    let hour = ((raw_hour % 24.0) + 24.0) % 24.0;
    let sun = sun_elevation(hour);
    let moon = moon_elevation(hour);
    let night = sun < 0.0;

    // Both rise ahead of the camera to its right, and set ahead of it to its left.
    let since_rise = if night {
        if hour < 6.0 {
            hour + 6.0
        } else {
            hour - 18.0
        }
    } else {
        hour - 6.0
    };
    let azimuth = (0.5 - since_rise / 12.0) * 80.0 * PI / 180.0;
    let elevation = (if night { moon } else { sun }).max(0.0) * PI / 180.0;
    let direction = [
        azimuth.sin() * elevation.cos(),
        elevation.sin(),
        azimuth.cos() * elevation.cos(),
    ];

    let sun_strength = smooth(0.0, 10.0, sun) * (0.6 + 0.4 * smooth(10.0, 40.0, sun));
    let moon_strength = 0.5 * smooth(0.0, 10.0, moon);
    let moonlight = [0.70, 0.80, 1.00];

    // Colours by the sun's elevation: night, twilight, sunrise or sunset, golden hour, day.
    let keys = [-12.0, -4.0, 2.0, 10.0, 32.0];
    let zenith = add(
        interpolate(
            sun,
            &keys,
            &[
                [0.016, 0.026, 0.060],
                [0.10, 0.12, 0.26],
                [0.38, 0.32, 0.40],
                [0.48, 0.41, 0.42],
                [0.20, 0.42, 0.85],
            ],
        ),
        scale(moonlight, moon_strength * 0.22),
    );
    let horizon = add(
        interpolate(
            sun,
            &keys,
            &[
                [0.030, 0.045, 0.090],
                [0.55, 0.30, 0.28],
                [1.00, 0.55, 0.25],
                [1.00, 0.64, 0.30],
                [0.62, 0.76, 0.95],
            ],
        ),
        scale(moonlight, moon_strength * 0.22),
    );
    let sun_colour = interpolate(
        sun,
        &keys,
        &[
            [1.00, 0.50, 0.20],
            [1.00, 0.50, 0.20],
            [1.00, 0.55, 0.22],
            [1.00, 0.68, 0.32],
            [1.00, 0.96, 0.88],
        ],
    );
    let ambient = interpolate(
        sun,
        &keys,
        &[[0.26; 3], [0.45; 3], [0.75; 3], [1.00; 3], [1.45; 3]],
    )[0]
        + moon_strength * 0.45;

    SkyState {
        hour,
        light_direction: direction,
        light_intensity: if night { moon_strength } else { sun_strength },
        light_colour: if night { moonlight } else { sun_colour },
        zenith,
        horizon,
        ambient,
        is_night: night,
    }
}

pub fn smooth(low: f64, high: f64, x: f64) -> f64 {
    let t = ((x - low) / (high - low)).max(0.0).min(1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn interpolate(x: f64, keys: &[f64], values: &[V3]) -> V3 {
    if x <= keys[0] {
        return values[0];
    }
    for i in 1..keys.len() {
        if x <= keys[i] {
            let t = (x - keys[i - 1]) / (keys[i] - keys[i - 1]);
            return add(values[i - 1], scale(add(values[i], scale(values[i - 1], -1.0)), t));
        }
    }
    values[values.len() - 1]
}
