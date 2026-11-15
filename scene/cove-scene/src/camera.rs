//! The camera on the seabed. Port of `SceneCamera` in `CoveScene.swift`.

use crate::model::scene_rules;

/// Where a point in the water (metres) appears: x and y run 0..1 across and up the picture (outside that
/// range the point is out of view), with its distance from the camera. `None` if it is behind the camera.
pub fn project(point: [f64; 3], aspect: f64) -> Option<(f64, f64, f64)> {
    let d = [point[0], point[1] + scene_rules::CAMERA_DEPTH, point[2]];
    let (cp, sp) = (scene_rules::CAMERA_PITCH.cos(), scene_rules::CAMERA_PITCH.sin());
    let up = d[1] * cp - d[2] * sp;
    let ahead = d[1] * sp + d[2] * cp;
    if ahead <= 0.3 {
        return None;
    }
    let f = scene_rules::CAMERA_FOCAL;
    Some((
        0.5 + f * d[0] / ahead / aspect,
        0.5 + f * up / ahead,
        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt(),
    ))
}
