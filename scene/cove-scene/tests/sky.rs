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

// The owner (2026-11-16): dawn and dusk last longer and colour the water warm.
#[test]
fn dawn_and_dusk_are_warm_and_long() {
    for h in [6.0, 18.0, 5.2, 6.8, 17.2, 18.8] {
        assert!(sky::state(h).warmth > 0.99, "warmth at {h}:00");
    }
    for h in [0.0, 3.0, 9.0, 12.0, 15.0, 21.0] {
        assert!(sky::state(h).warmth < 1e-9, "warmth at {h}:00");
    }
    assert!(sky::state(8.0).warmth > 0.3 && sky::state(16.0).warmth > 0.3);
}

#[test]
fn sun_stays_low_longer() {
    // A plain sine reaches 10 degrees by 6:40; the low sun lasts past 7:00.
    assert!(sky::sun_elevation(7.0) < 10.0);
    assert!((sky::sun_elevation(12.0) - sky::SUN_PEAK_ELEVATION).abs() < 1e-9);
    assert!(sky::sun_elevation(3.0) < 0.0 && sky::sun_elevation(21.0) < 0.0);
}
