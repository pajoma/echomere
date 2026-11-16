mod common;
use common::*;
use cove_scene::sky;

#[test]
fn sun_at_noon_moon_at_midnight() {
    let noon = sky::state(12.0);
    let midnight = sky::state(0.0);
    assert!(!noon.is_night && noon.light_direction[1] > 0.8);
    assert!(midnight.is_night && midnight.light_direction[1] > 0.6);
}

#[test]
fn night_darker_never_black() {
    let noon = sky::state(12.0);
    let midnight = sky::state(0.0);
    assert!(midnight.ambient < noon.ambient / 2.5);
    assert!(midnight.ambient > 0.2);
    assert!(midnight.light_intensity < noon.light_intensity);
}

#[test]
fn no_light_on_horizon() {
    assert!(sky::state(6.0).light_intensity < 1e-9);
    assert!(sky::state(18.0).light_intensity < 1e-9);
}

#[test]
fn light_never_jumps() {
    let max3 = |a: [f64; 3], b: [f64; 3]| {
        (0..3)
            .map(|i| a[i] - b[i])
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let mut largest: f64 = 0.0;
    for minute in 0..(24 * 60) {
        let a = sky::state(minute as f64 / 60.0);
        let b = sky::state((minute + 1) as f64 / 60.0);
        largest = largest
            .max((a.ambient - b.ambient).abs())
            .max((a.light_intensity - b.light_intensity).abs())
            .max(max3(a.zenith, b.zenith))
            .max(max3(a.horizon, b.horizon));
    }
    println!("largest jump per minute {largest:.4}");
    assert!(largest < 0.05);
}

#[test]
fn sun_and_moon_within_lens() {
    let mut h = 0.0;
    while h < 24.0 {
        let d = sky::state(h).light_direction;
        assert!(d[0].atan2(d[2]).abs() <= 41.0_f64.to_radians(), "hour {h}");
        h += 0.25;
    }
}

#[test]
fn sky_hour_is_utc_and_wraps() {
    assert!((sky::hour(NOON) - (NOON / HOUR) % 24.0).abs() < 1e-9);
    assert!(sky::hour(-1.0) >= 0.0 && sky::hour(-1.0) < 24.0);
    assert_eq!(sky::state_at(NOON), sky::state(sky::hour(NOON)));
    let _ = DAY;
}
