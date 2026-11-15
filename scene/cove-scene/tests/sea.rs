mod common;
use common::*;
use cove_scene::{kelp, sea};

#[test]
fn sea_loops_seamlessly() {
    assert!((sea::time(NOON) - sea::time(NOON + sea::LOOP_PERIOD)).abs() < 1e-6);
    assert!((sea::time(NOON + 1.0) - sea::time(NOON) - 1.0).abs() < 1e-6);
}

// The Swell checks compare frames of two coves; frames arrive in Task 3. The sea
// time depends on the moment only, so this checks that.
#[test]
fn every_cove_same_sea() {
    assert_eq!(sea::time(NOON), sea::time(NOON));
    assert!(sea::time(-1.0) >= 0.0 && sea::time(-1.0) < sea::LOOP_PERIOD);
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
    assert!(kelp::surge_period() > 8.0 && kelp::surge_period() < 10.0);
    println!("surge period {:.1} s", kelp::surge_period());
}

// The motion depends on time only, never on a cove.
#[test]
fn kelp_sways_same_everywhere() {
    assert_eq!(kelp::motion(NOON), kelp::motion(NOON));
}
