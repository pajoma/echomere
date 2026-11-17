//! The schedule of fish school visits: a pure function of the cove seed and the time, so every screen in
//! the cove sees the same school at the same moment (ADR 0009).
//!
//! Time is cut into slots per kind of school, and each slot holds one visit. A visit is a route for the
//! school's lead point: in from beyond one edge of the picture, out beyond the other, and for some
//! baitfish visits a few rounds of a circle in the picture's middle on the way.

use crate::model::{scene_rules, CoveSeed};
use crate::random::{combine, SplitMix64};
use std::cmp::Ordering;

/// How fast the lead swims along its route outside the circling, in metres per second.
pub const LEAD_SPEED: f64 = 0.9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishKind {
    /// The big school of small fish: 300 to 400 of them.
    Baitfish,
    /// The small school of larger fish: 30 to 60 of them.
    Mackerel,
}

impl FishKind {
    fn code(self) -> u64 {
        match self {
            FishKind::Baitfish => 1,
            FishKind::Mackerel => 2,
        }
    }

    /// Seconds from the start of one slot to the next.
    fn slot(self) -> f64 {
        match self {
            FishKind::Baitfish => 450.0,
            FishKind::Mackerel => 300.0,
        }
    }

    /// How late in its slot a visit may start, in seconds.
    fn slack(self) -> f64 {
        match self {
            FishKind::Baitfish => 135.0,
            FishKind::Mackerel => 60.0,
        }
    }

    /// How far the lead swims on past the exit point, about one school length, so that the fish
    /// lagging behind it are out of sight when the visit ends.
    fn overshoot(self) -> f64 {
        match self {
            FishKind::Baitfish => 20.0,
            FishKind::Mackerel => 10.0,
        }
    }
}

/// A few rounds of a circle in the water, on the way (the school circles at its centre, in a horizontal plane).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circling {
    pub start: f64,
    pub end: f64,
    pub centre: [f64; 3],
    pub radius: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchoolVisit {
    /// Stable per visit (a hash of the seed, the kind and the slot).
    pub id: u64,
    pub kind: FishKind,
    pub count: usize,
    /// Seconds since 1970. The lead is out of sight at both.
    pub start: f64,
    pub end: f64,
    pub entry: [f64; 3],
    pub exit: [f64; 3],
    pub circling: Option<Circling>,
}

type Vec3 = [f64; 3];

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn length(a: Vec3) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

fn lerp(a: Vec3, b: Vec3, f: f64) -> Vec3 {
    [
        a[0] + (b[0] - a[0]) * f,
        a[1] + (b[1] - a[1]) * f,
        a[2] + (b[2] - a[2]) * f,
    ]
}

/// A point `distance` along the line from `from` to `to`, and on beyond `to` straight sideways at the depth
/// and distance of `to`, away from the picture (so the lead stays out of sight however the last leg slopes).
fn along(from: Vec3, to: Vec3, distance: f64) -> Vec3 {
    let total = length(sub(to, from));
    if distance <= total {
        return lerp(from, to, if total > 0.0 { distance / total } else { 1.0 });
    }
    let away = if to[0] < 0.0 { -1.0 } else { 1.0 };
    [to[0] + away * (distance - total), to[1], to[2]]
}

impl Circling {
    /// Where the lead joins the circle: the point of it nearest to where the visit came from.
    fn join(&self, entry: Vec3) -> Vec3 {
        let (dx, dz) = (entry[0] - self.centre[0], entry[2] - self.centre[2]);
        let d = dx.hypot(dz).max(1e-9);
        [
            self.centre[0] + self.radius * dx / d,
            self.centre[1],
            self.centre[2] + self.radius * dz / d,
        ]
    }

    /// The way round: towards the side the school is going to, so the circle opens up to the exit.
    fn direction(entry: Vec3) -> f64 {
        if entry[0] < 0.0 {
            1.0
        } else {
            -1.0
        }
    }

    /// The point of the circle `time` seconds after its start.
    fn at(&self, entry: Vec3, time: f64) -> Vec3 {
        let join = self.join(entry);
        let angle = (join[2] - self.centre[2]).atan2(join[0] - self.centre[0])
            + Self::direction(entry) * LEAD_SPEED * time / self.radius;
        [
            self.centre[0] + self.radius * angle.cos(),
            self.centre[1],
            self.centre[2] + self.radius * angle.sin(),
        ]
    }
}

impl SchoolVisit {
    /// The route's lead point at a moment (clamped to the visit): the school follows it.
    pub fn lead(&self, at: f64) -> [f64; 3] {
        let t = at.max(self.start).min(self.end);
        match &self.circling {
            None => along(self.entry, self.exit, (t - self.start) * LEAD_SPEED),
            Some(c) => {
                if t <= c.start {
                    lerp(
                        self.entry,
                        c.join(self.entry),
                        (t - self.start) / (c.start - self.start).max(1e-9),
                    )
                } else if t <= c.end {
                    c.at(self.entry, t - c.start)
                } else {
                    along(
                        c.at(self.entry, c.end - c.start),
                        self.exit,
                        (t - c.end) * LEAD_SPEED,
                    )
                }
            }
        }
    }
}

/// A point `ahead` metres along the view axis at `depth` metres, `side` x `ahead` to the right.
fn point(ahead: f64, depth: f64, sideways: f64) -> Vec3 {
    let (cp, sp) = (
        scene_rules::CAMERA_PITCH.cos(),
        scene_rules::CAMERA_PITCH.sin(),
    );
    [
        sideways * ahead,
        -depth,
        (ahead - (scene_rules::CAMERA_DEPTH - depth) * sp) / cp,
    ]
}

fn visit_in_slot(seed: CoveSeed, kind: FishKind, slot: i64) -> SchoolVisit {
    let id = combine(combine(seed.0, kind.code()), slot as u64);
    let mut random = SplitMix64::new(id);
    let start = slot as f64 * kind.slot() + random.range(0.0, kind.slack());
    let count = match kind {
        FishKind::Baitfish => random.int(300, 400),
        FishKind::Mackerel => random.int(30, 60),
    } as usize;
    // In from one side, out of the other, both beyond the edge of the picture at any shape (21:9 too).
    let side = if random.next() & 1 == 0 { -1.0 } else { 1.0 };
    let entry = point(
        random.range(12.0, 30.0),
        random.range(8.0, 24.0),
        side * 2.2,
    );
    let exit = point(
        random.range(12.0, 30.0),
        random.range(8.0, 24.0),
        -side * 2.2,
    );
    let circles = kind == FishKind::Baitfish && random.unit() < 0.4;
    let circle_time = random.range(60.0, 120.0);
    let circle_centre = point(
        random.range(20.0, 26.0),
        random.range(8.0, 24.0),
        random.range(-0.1, 0.1),
    );
    let radius = random.range(6.0, 10.0);

    let mut visit = SchoolVisit {
        id,
        kind,
        count,
        start,
        end: start,
        entry,
        exit,
        circling: None,
    };
    let overshoot = kind.overshoot();
    if circles {
        let mut c = Circling {
            start: 0.0,
            end: 0.0,
            centre: circle_centre,
            radius,
        };
        c.start = start + length(sub(c.join(entry), entry)) / LEAD_SPEED;
        c.end = c.start + circle_time;
        let leave = c.at(entry, circle_time);
        visit.end = c.end + (length(sub(exit, leave)) + overshoot) / LEAD_SPEED;
        visit.circling = Some(c);
    } else {
        // A mackerel visit is at most 160 s long and starts at most 60 s into its 300 s slot, so it
        // always ends more than 30 s before the next slot's earliest start; no shortening is needed.
        visit.end = start + (length(sub(exit, entry)) + overshoot) / LEAD_SPEED;
    }
    visit
}

/// The visits under way at `at`, ordered by start.
pub fn visits(seed: CoveSeed, at: f64) -> Vec<SchoolVisit> {
    let mut found = Vec::new();
    for kind in [FishKind::Baitfish, FishKind::Mackerel] {
        let slot = (at / kind.slot()).floor() as i64;
        for s in [slot - 1, slot] {
            let v = visit_in_slot(seed, kind, s);
            if v.start <= at && at < v.end {
                found.push(v);
            }
        }
    }
    found.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(Ordering::Equal)
            .then(a.id.cmp(&b.id))
    });
    found
}
