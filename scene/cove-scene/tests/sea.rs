mod common;
use common::*;
use cove_scene::model::CoveSeed;
use cove_scene::scene::CoveScene;
use cove_scene::{kelp, sea};

#[test]
fn sea_loops_seamlessly() {
    let repeat = sea::LOOP_PERIOD / sea::SEA_PACE;
    assert!((sea::time(NOON) - sea::time(NOON + repeat)).abs() < 1e-6);
    assert!((sea::time(NOON + 1.0) - sea::time(NOON) - sea::SEA_PACE).abs() < 1e-6);
}

#[test]
fn every_cove_same_sea() {
    let a = CoveScene::new(CoveSeed(1)).frame(NOON, &[], &[]).sea_time;
    let b = CoveScene::new(CoveSeed(2)).frame(NOON, &[], &[]).sea_time;
    assert_eq!(a, b);
}

#[test]
fn kelp_sways_with_swell() {
    let sway = kelp::motion(NOON);
    let later = kelp::motion(NOON + kelp::surge_period() / 4.0);
    let mut quarter = later.surge - sway.surge;
    if quarter < 0.0 {
        quarter += 2.0 * std::f64::consts::PI;
    }
    assert!((quarter - std::f64::consts::FRAC_PI_2).abs() < 1e-6);
    // A long swell, slowed with the sea (owner, 2026-11-16: meditative, the waves "träger"); the Swift
    // reference had 8 to 10 s.
    assert!(kelp::surge_period() > 40.0 && kelp::surge_period() < 48.0);
    println!("surge period {:.1} s", kelp::surge_period());
}

#[test]
fn kelp_sways_same_everywhere() {
    let a = CoveScene::new(CoveSeed(4711))
        .frame(NOON, &[], &[])
        .kelp_motion;
    let b = CoveScene::new(CoveSeed(9))
        .frame(NOON, &[], &[])
        .kelp_motion;
    assert_eq!(a, b);
}

// The owner (2026-11-16): the waves must move much slower.
#[test]
fn sea_runs_slow() {
    assert!((sea::SEA_PACE - 0.25).abs() < 1e-12);
}

#[test]
fn kelp_clock_runs_slow() {
    let a = kelp::motion(NOON);
    let b = kelp::motion(NOON + 1.0);
    assert!((b.time - a.time - kelp::KELP_PACE).abs() < 1e-9);
    assert!(
        (kelp::motion(NOON + 86_400.0).time - a.time).abs() < 1e-6,
        "repeats daily"
    );
    assert!((kelp::KELP_PACE - 0.4).abs() < 1e-12);
}
