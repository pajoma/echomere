mod common;
use common::*;
use cove_scene::model::CoveSeed;
use cove_scene::scene::CoveScene;
use cove_scene::sky;

#[test]
fn kelp_grows_where_giant_kelp_grows() {
    let kelp = munich_bay().layout.kelp;
    assert!(
        !kelp.is_empty()
            && kelp.iter().all(|k| k.base_depth >= 15.0
                && k.base_depth <= 30.0
                && k.length > k.base_depth
                && k.length < 46.0)
    );
}

#[test]
fn kelp_is_only_hinted_at() {
    assert!(munich_bay().layout.kelp.iter().all(|k| k.base_z > 50.0));
}

#[test]
fn small_fish_swim_near_the_camera() {
    let s = munich_bay().layout.schools;
    assert!(!s.is_empty() && s.iter().all(|s| s.depth < 0.4));
}

#[test]
fn small_fish_are_drawn_far_to_near() {
    let f = munich_bay().frame(NOON, &guests(), &traces());
    assert!(f.creatures.windows(2).all(|w| w[0].depth >= w[1].depth));
}

#[test]
fn small_fish_move_smoothly_from_one_frame_to_the_next() {
    let scene = munich_bay();
    let f = scene.frame(NOON, &guests(), &traces());
    let n = scene.frame(NOON + 1.0 / 15.0, &guests(), &traces());
    let mut largest = 0.0f64;
    let mut matched = 0;
    for c in &f.creatures {
        if let Some(o) = n.creatures.iter().find(|o| o.id == c.id) {
            matched += 1;
            largest = largest.max((c.x - o.x).hypot(c.y - o.y));
        }
    }
    println!("largest step {largest:.5} (<0.01)");
    assert!(largest < 0.01 && matched == f.creatures.len());
}

#[test]
fn small_fish_swim_in_schools_of_twenty_or_more() {
    let f = munich_bay().frame(NOON, &guests(), &traces());
    assert!(
        !f.creatures.is_empty()
            && munich_bay()
                .layout
                .schools
                .iter()
                .all(|s| s.members.len() >= 20)
    );
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
