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
