//! Everything about a cove that is generated once from its seed: the kelp.
//! Port of `CoveLayout.swift`; the kelp is Echomere's own (a frame of blades at the picture's edges).

use crate::kelp::KelpBlade;
use crate::model::{scene_rules, CoveSeed};
use crate::random::SplitMix64;

/// Mixed into the cove seed for the kelp's own random stream ("kelpkelp").
const KELP_STREAM: u64 = 0x6B65_6C70_6B65_6C70;
/// The picture shape the kelp is placed for; the kelp shader turns the blades for other shapes.
const REFERENCE_ASPECT: f64 = 1.6;

#[derive(Debug, Clone, PartialEq)]
pub struct CoveLayout {
    pub kelp: Vec<KelpBlade>,
}

impl CoveLayout {
    pub fn new(seed: CoveSeed) -> Self {
        Self {
            kelp: Self::blades_for(seed),
        }
    }

    /// The kelp frame: 3 to 6 long blades on each side, rooted below the picture 2 to 8 m ahead of the
    /// camera, 4 to 14 % in from the edges of the 16:10 reference picture, heading for 8 to 16 % in. Drawn from their own stream of the cove seed.
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
}
