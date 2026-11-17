//! Everything about a cove that is generated once from its seed: kelp and ambient life.
//! Port of `CoveLayout.swift`; the kelp is Echomere's own (a frame of blades at the picture's edges).

use crate::kelp::KelpBlade;
use crate::model::{scene_rules, CoveSeed};
use crate::random::SplitMix64;

/// Mixed into the cove seed for the kelp's own random stream ("kelpkelp").
const KELP_STREAM: u64 = 0x6B65_6C70_6B65_6C70;
/// The picture shape the kelp is placed for; the kelp shader turns the blades for other shapes.
const REFERENCE_ASPECT: f64 = 1.6;

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
    pub kelp: Vec<KelpBlade>,
    pub schools: Vec<FishSchool>,
}

impl CoveLayout {
    pub fn new(seed: CoveSeed) -> Self {
        Self {
            kelp: Self::blades_for(seed),
            schools: Self::schools_for(seed),
        }
    }

    /// The kelp frame: 3 to 6 long blades on each side, rooted below the picture 2 to 8 m ahead of the
    /// camera, 4 to 14 % in from the edges of the 16:10 reference picture, heading for 8 to 16 % in. Drawn from their own stream of the cove
    /// seed, so the fish don't depend on them.
    pub fn blades_for(seed: CoveSeed) -> Vec<KelpBlade> {
        let mut random = SplitMix64::new(seed.0 ^ KELP_STREAM);
        let (cp, sp) = (
            scene_rules::CAMERA_PITCH.cos(),
            scene_rules::CAMERA_PITCH.sin(),
        );
        let focal = scene_rules::CAMERA_FOCAL;
        let mut kelp = Vec::new();
        for side in [-1.0, 1.0] {
            let blades = random.int(3, 6);
            for _ in 0..blades {
                // Where the root appears (picture x and y, y below the bottom edge) and how far ahead it is.
                let px = 0.5 + side * (0.5 - random.range(0.04, 0.14));
                let py = random.range(-0.6, -0.2);
                let ahead = random.range(2.0, 8.0);
                // Back from the picture into the water (the inverse of `camera::project` at 16:10).
                let dx = (px - 0.5) * ahead * REFERENCE_ASPECT / focal;
                let up = (py - 0.5) * ahead / focal;
                let dy = up * cp + ahead * sp;
                let dz = -up * sp + ahead * cp;
                // The picture x the blade's line heads for, a little further in than its root: the blade
                // curls outwards as it rises, back towards the edge.
                let line_x = 0.5 + side * (0.5 - random.range(0.08, 0.16));
                let lean = (line_x - 0.5) * REFERENCE_ASPECT / focal;
                kelp.push(KelpBlade {
                    root: [dx, dy - scene_rules::CAMERA_DEPTH, dz],
                    lean,
                    length: random.range(8.0, 20.0),
                    forward: random.range(-0.2, 0.2),
                    width: random.range(0.6, 1.1),
                    curvature: random.unit(),
                    seed: [random.unit(), random.unit()],
                    side,
                    sway_phase: random.angle(),
                });
            }
        }
        kelp
    }

    /// The schools of small fish.
    pub fn schools_for(seed: CoveSeed) -> Vec<FishSchool> {
        let mut random = SplitMix64::new(seed.0);
        let school_count = random.int(2, 3);
        (0..school_count)
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
            .collect()
    }
}
