mod common;
use common::*;
use cove_scene::camera;
use cove_scene::model::CoveSeed;
use cove_scene::scene::CoveScene;
use cove_scene::sky;

#[test]
fn kelp_frames_the_picture() {
    let kelp = munich_bay().layout.kelp;
    for side in [-1.0, 1.0] {
        let n = kelp.iter().filter(|b| b.side == side).count();
        assert!((3..=6).contains(&n), "{n} blades on side {side}");
    }
    for b in &kelp {
        let (x, y, _) = camera::project(b.root, 1.6).expect("root in front of the camera");
        assert!(y < 0.0, "root below the picture, at y {y}");
        assert!(
            if b.side < 0.0 { x < 0.2 } else { x > 0.8 },
            "root at picture x {x}"
        );
        let ahead = camera::ahead(b.root);
        assert!((2.0..=8.0).contains(&ahead), "root {ahead} m ahead");
        assert!((8.0..=20.0).contains(&b.length));
    }
}

#[test]
fn scene_uses_its_own_sky() {
    assert_eq!(munich_bay().frame(NOON, &[], &[]).sky, sky::state_at(NOON));
}

#[test]
fn camera_projects_ahead_and_rejects_behind() {
    use cove_scene::camera;
    assert!(camera::project([0.0, -30.0, 50.0], 1.6).is_some());
    assert!(camera::project([0.0, -30.0, -5.0], 1.6).is_none());
    let _ = CoveScene::new(CoveSeed(1));
}
