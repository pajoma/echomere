//! The scene's data model. Port of `Model.swift`.

/// The seed a cove's scene is generated from. It comes from the backend and never changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoveSeed(pub u64);

/// The kinds of whale a creator can choose from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WhaleSpecies {
    // The pod seed hashes `species as i64`: never reorder or renumber these.
    SouthernRightWhale = 0,
    Orca = 1,
}

impl WhaleSpecies {
    pub const ALL: [WhaleSpecies; 2] = [WhaleSpecies::SouthernRightWhale, WhaleSpecies::Orca];

    /// Length of an average adult, in metres.
    pub fn length(self) -> f64 {
        match self {
            WhaleSpecies::SouthernRightWhale => 14.5,
            WhaleSpecies::Orca => 7.0,
        }
    }

    /// How long one cycle of diving and breathing lasts, in seconds (closed range).
    pub fn breathing_cycle(self) -> (f64, f64) {
        match self {
            WhaleSpecies::SouthernRightWhale => (420.0, 720.0),
            WhaleSpecies::Orca => (150.0, 240.0),
        }
    }

    /// Whether whales of this species travel together as a pod.
    pub fn travels_in_pods(self) -> bool {
        self == WhaleSpecies::Orca
    }

    /// How fast it swims compared with an orca. Right whales are slow.
    pub fn pace(self) -> f64 {
        match self {
            WhaleSpecies::SouthernRightWhale => 1.0 / 1.8,
            WhaleSpecies::Orca => 1.0,
        }
    }

    /// How many breaths it takes at the surface before it dives again (closed range).
    pub fn breaths(self) -> (i64, i64) {
        match self {
            WhaleSpecies::SouthernRightWhale => (4, 6),
            WhaleSpecies::Orca => (3, 5),
        }
    }

    /// Seconds from one breath to the next.
    pub fn breath_interval(self) -> f64 {
        match self {
            WhaleSpecies::SouthernRightWhale => 20.0,
            WhaleSpecies::Orca => 14.0,
        }
    }
}

/// How a whale looks, chosen by its creator from fixed options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhaleLook {
    pub species: WhaleSpecies,
    /// Relative to an average adult of its species.
    pub size: f64,
}

impl WhaleLook {
    pub const SIZE_RANGE: (f64, f64) = (0.8, 1.2);

    /// Clamps `size` to `SIZE_RANGE`, as the Swift initialiser does.
    pub fn new(species: WhaleSpecies, size: f64) -> Self {
        Self {
            species,
            size: size.max(Self::SIZE_RANGE.0).min(Self::SIZE_RANGE.1),
        }
    }

    /// Length in metres.
    pub fn length(&self) -> f64 {
        self.species.length() * self.size
    }
}

/// A creature visiting the cove during its current stop.
#[derive(Debug, Clone, PartialEq)]
pub struct Visitor {
    pub creature_id: String,
    pub look: WhaleLook,
    /// Seconds since 1970.
    pub arrived_at: f64,
}

/// The trace a creature left in the cove when it moved on.
#[derive(Debug, Clone, PartialEq)]
pub struct Echo {
    pub creature_id: String,
    pub look: WhaleLook,
    /// Seconds since 1970.
    pub left_at: f64,
}

/// The rules the scene follows (see the version 1 spec, #2, and ADR 0005).
pub mod scene_rules {
    /// Five visitors, plus the creator's own new whale on its first day.
    pub const MAX_WHALES: usize = 6;
    pub const MAX_ECHOES: usize = 5;
    pub const ECHO_LIFETIME: f64 = 7.0 * 24.0 * 3600.0;
    pub const WHALE_FADE_IN: f64 = 120.0;
    /// The scene is 1.6 units wide and 1 unit high. Screens with another aspect ratio see more or
    /// less of the sea to the left and right, always centred on `WORLD_CENTRE_X`.
    pub const WORLD_WIDTH: f64 = 1.6;
    pub const WORLD_CENTRE_X: f64 = 0.8;
    /// The camera stands on the seabed this far below the surface, in metres. Must match
    /// `CAMERA_DEPTH` in the shader.
    pub const CAMERA_DEPTH: f64 = 30.0;
    /// The camera is tilted up by this angle (radians) and has this focal length, in picture
    /// heights. Must match `PITCH` and `FOCAL` in the shader.
    pub const CAMERA_PITCH: f64 = 0.69813; // 40 degrees up
    pub const CAMERA_FOCAL: f64 = 0.714;
}
