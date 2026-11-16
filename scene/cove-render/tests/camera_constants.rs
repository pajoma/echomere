//! The camera is defined twice: in the scene logic (which whales are in view, where kelp stands) and in the
//! shaders (what is drawn). These tests keep the two in step, and pin the owner's choice of tilt.
use cove_scene::model::scene_rules;

const COMMON: &str = include_str!("../shaders/common.wgsl");

fn wgsl_const(name: &str) -> f64 {
    let prefix = format!("const {name}: f32 = ");
    let line = COMMON
        .lines()
        .find(|l| l.starts_with(&prefix))
        .unwrap_or_else(|| panic!("{name} not found in common.wgsl"));
    line[prefix.len()..]
        .split(';')
        .next()
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

#[test]
fn camera_looks_up_forty_degrees() {
    assert!((scene_rules::CAMERA_PITCH - 40f64.to_radians()).abs() < 1e-4);
}

#[test]
fn shader_camera_matches_scene_camera() {
    assert!((wgsl_const("PITCH") - scene_rules::CAMERA_PITCH).abs() < 1e-4);
    assert!((wgsl_const("FOCAL") - scene_rules::CAMERA_FOCAL).abs() < 1e-4);
    let horizon = 0.5 - scene_rules::CAMERA_FOCAL * scene_rules::CAMERA_PITCH.tan();
    assert!(
        (wgsl_const("HORIZON") - horizon).abs() < 1e-3,
        "HORIZON must be 0.5 - FOCAL * tan(PITCH)"
    );
}

#[test]
fn exposure_is_the_darker_setting() {
    assert!((wgsl_const("EXPOSURE") - 0.13).abs() < 1e-6);
}
