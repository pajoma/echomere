mod common;
use common::*;
use cove_scene::model::CoveSeed;
use cove_scene::scene::CoveScene;

#[test]
fn frame_is_reproducible_across_constructions() {
    let a = CoveScene::new(CoveSeed(4711)).frame(NOON, &guests(), &traces());
    let b = CoveScene::new(CoveSeed(4711)).frame(NOON, &guests(), &traces());
    assert_eq!(a, b);
}

#[test]
fn same_cove_visitors_echoes_and_time_give_the_same_frame() {
    assert_eq!(
        CoveScene::new(CoveSeed(4711)).frame(NOON, &guests(), &traces()),
        munich_bay().frame(NOON, &guests(), &traces())
    );
}

#[test]
fn visitor_order_does_not_change_the_frame() {
    let mut g = guests();
    g.reverse();
    let mut t = traces();
    t.reverse();
    assert_eq!(
        munich_bay().frame(NOON, &g, &t),
        munich_bay().frame(NOON, &guests(), &traces())
    );
}

#[test]
fn different_coves_have_different_kelp_and_fish() {
    assert_ne!(
        CoveScene::new(CoveSeed(1)).layout,
        CoveScene::new(CoveSeed(2)).layout
    );
}

#[test]
fn same_whale_swims_a_different_path_in_another_cove() {
    let g = guests();
    assert_ne!(
        CoveScene::new(CoveSeed(1)).frame(NOON, &g[..1], &[]).whales,
        CoveScene::new(CoveSeed(2)).frame(NOON, &g[..1], &[]).whales
    );
}
