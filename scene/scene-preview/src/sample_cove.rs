//! Made-up visitors and echoes for looking at the scene before the backend exists. The port of
//! `SampleCove.swift`.

use cove_scene::model::{Echo, Visitor, WhaleLook, WhaleSpecies};

pub fn visitors(seed: u64, count: usize, now: f64) -> Vec<Visitor> {
    (0..count)
        .map(|i| Visitor {
            creature_id: format!("sample-{seed}-{i}"),
            // Orcas and right whales in turn, so a cove of five has a pod of three orcas.
            look: WhaleLook::new(
                if i % 2 == 0 {
                    WhaleSpecies::Orca
                } else {
                    WhaleSpecies::SouthernRightWhale
                },
                0.85 + 0.12 * (i % 3) as f64,
            ),
            // The last one arrives a moment after start, to show the fade-in.
            arrived_at: if i == count - 1 && count > 1 {
                now + 5.0
            } else {
                now - 3600.0 * (i + 1) as f64
            },
        })
        .collect()
}

pub fn echoes(seed: u64, count: usize, now: f64) -> Vec<Echo> {
    (0..count)
        .map(|i| Echo {
            creature_id: format!("echo-{seed}-{i}"),
            look: WhaleLook::new(WhaleSpecies::SouthernRightWhale, 1.0),
            left_at: now - 86_400.0 * (i as f64 + 0.3),
        })
        .collect()
}
