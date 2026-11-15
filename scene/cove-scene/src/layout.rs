//! Everything about a cove that is generated once from its seed: kelp and ambient life.
//! Port of `CoveLayout.swift`. The order of random draws matches the Swift code exactly.

use crate::kelp::KelpFrond;
use crate::model::{scene_rules, CoveSeed};
use crate::random::SplitMix64;

#[derive(Debug, Clone, PartialEq)]
pub struct FishMember {
    pub offset_x: f64,
    pub offset_y: f64,
    pub wobble_phase: f64,
    pub size: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FishSchool {
    pub depth: f64,
    pub size: f64,
    /// 0 is silver, 3 is golden.
    pub tint: i64,
    pub centre_x: f64,
    pub centre_y: f64,
    pub amplitude_x: f64,
    pub amplitude_y: f64,
    pub period_x: f64,
    pub period_y: f64,
    pub phase_x: f64,
    pub phase_y: f64,
    pub members: Vec<FishMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoveLayout {
    pub kelp: Vec<KelpFrond>,
    pub schools: Vec<FishSchool>,
}

impl CoveLayout {
    pub fn new(seed: CoveSeed) -> Self {
        let mut random = SplitMix64::new(seed.0);

        // Giant kelp, only hinted at in the background, rooted 25 to 30 m down on ground that rises
        // gently in the distance.
        let mut kelp = Vec::new();
        let plants = random.int(5, 7);
        for plant in 0..plants {
            let z = 55.0 + 45.0 * (plant as f64 + random.unit()) / plants as f64;
            let x = random.range(-0.95, 0.95) * z;
            let depth = scene_rules::CAMERA_DEPTH - 0.1 * (z - 55.0);
            let plant_phase = random.angle();
            let fronds = random.int(2, 4);
            for _ in 0..fronds {
                let base_x = x + random.range(-0.12, 0.12);
                let base_z = z + random.range(-0.12, 0.12);
                let length = depth + random.range(2.0, 7.0);
                let sway_phase = plant_phase + random.range(-0.25, 0.25);
                let twist_phase = random.angle();
                let blade_seed = random.range(0.0, 100.0);
                let blade_scale = random.range(0.85, 1.2);
                kelp.push(KelpFrond {
                    base_x,
                    base_z,
                    base_depth: depth,
                    length,
                    sway_phase,
                    twist_phase,
                    blade_seed,
                    blade_scale,
                });
            }
        }

        let school_count = random.int(2, 3);
        let schools = (0..school_count)
            .map(|_| {
                // Fields are drawn in the order the Swift initialiser lists them.
                let depth = random.range(0.05, 0.35);
                let size = random.range(0.8, 1.3);
                let tint = random.int(0, 3);
                let centre_x = random.range(0.3, 1.3);
                let centre_y = random.range(0.38, 0.75);
                let amplitude_x = random.range(0.3, 0.7);
                let amplitude_y = random.range(0.04, 0.1);
                let period_x = random.range(70.0, 200.0);
                let period_y = random.range(30.0, 80.0);
                let phase_x = random.angle();
                let phase_y = random.angle();
                let member_count = random.int(20, 34);
                let members = (0..member_count)
                    .map(|_| FishMember {
                        offset_x: random.range(-0.10, 0.10),
                        offset_y: random.range(-0.045, 0.045),
                        wobble_phase: random.angle(),
                        size: random.range(0.85, 1.15),
                    })
                    .collect();
                FishSchool {
                    depth,
                    size,
                    tint,
                    centre_x,
                    centre_y,
                    amplitude_x,
                    amplitude_y,
                    period_x,
                    period_y,
                    phase_x,
                    phase_y,
                    members,
                }
            })
            .collect();

        Self { kelp, schools }
    }
}
