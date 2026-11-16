#![allow(dead_code)]
//! Shared helpers, ported from `SceneChecks/main.swift`.

use cove_scene::model::{Echo, Visitor, WhaleLook, WhaleSpecies};

pub const NOON: f64 = 1_794_000_000.0;
pub const HOUR: f64 = 3600.0;
pub const DAY: f64 = 24.0 * HOUR;

pub fn visitor(id: &str, arrived: f64, species: WhaleSpecies) -> Visitor {
    Visitor {
        creature_id: id.to_string(),
        look: WhaleLook::new(species, 1.0),
        arrived_at: arrived,
    }
}

pub fn echo(id: &str, left: f64) -> Echo {
    Echo {
        creature_id: id.to_string(),
        look: WhaleLook::new(WhaleSpecies::SouthernRightWhale, 1.0),
        left_at: left,
    }
}

use cove_scene::model::CoveSeed;
use cove_scene::scene::{CoveScene, WhalePose};

pub fn munich_bay() -> CoveScene {
    CoveScene::new(CoveSeed(4_711))
}

pub fn guests() -> Vec<Visitor> {
    vec![
        visitor(
            "whale-a",
            NOON - 5.0 * HOUR,
            WhaleSpecies::SouthernRightWhale,
        ),
        visitor("whale-b", NOON - 2.0 * HOUR, WhaleSpecies::Orca),
        visitor("whale-c", NOON - HOUR, WhaleSpecies::SouthernRightWhale),
    ]
}

pub fn traces() -> Vec<Echo> {
    vec![
        echo("whale-x", NOON - DAY),
        echo("whale-y", NOON - 3.0 * DAY),
    ]
}

/// Swift `stride(from: 0, to: end, by: step)` for whole-number steps.
pub fn stride(end: f64, step: f64) -> impl Iterator<Item = f64> {
    let count = (end / step).ceil() as u64;
    (0..count).map(move |i| i as f64 * step)
}

pub fn dist(a: &WhalePose, b: &WhalePose) -> f64 {
    let d = [
        a.position[0] - b.position[0],
        a.position[1] - b.position[1],
        a.position[2] - b.position[2],
    ];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn track(id: &str, among: &[Visitor], start: f64, seconds: f64, every: f64) -> Vec<WhalePose> {
    let scene = munich_bay();
    stride(seconds, every)
        .filter_map(|t| {
            scene
                .frame(start + t, among, &[])
                .whales
                .into_iter()
                .find(|w| w.id == id)
        })
        .collect()
}

pub fn speed_of(poses: &[WhalePose], every: f64) -> f64 {
    let total: f64 = poses.windows(2).map(|w| dist(&w[0], &w[1]) / every).sum();
    total / (poses.len().saturating_sub(1)).max(1) as f64
}

pub fn pod() -> Vec<Visitor> {
    vec![
        visitor("orca-1", NOON - 3.0 * HOUR, WhaleSpecies::Orca),
        visitor("orca-2", NOON - 2.0 * HOUR, WhaleSpecies::Orca),
        visitor("orca-3", NOON - HOUR, WhaleSpecies::Orca),
        visitor("right-1", NOON - HOUR, WhaleSpecies::SouthernRightWhale),
    ]
}
