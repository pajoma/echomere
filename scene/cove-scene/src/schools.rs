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
    /// A time of roaming around the camera, for visits that don't circle: the school swims in and out of
    /// the picture before it leaves the cove (owner, 2026-11-17).
    pub roaming: Option<Roaming>,
}

/// The lead point roaming around the camera from stop to stop, lingering at each: the school wanders in and
/// out of the picture on an erratic path and mills where the lead lingers (owner, 2026-11-18: "it should
/// linger a bit").
#[derive(Debug, Clone, PartialEq)]
pub struct Roaming {
    pub start: f64,
    pub end: f64,
    pub stops: Vec<Stop>,
}

/// One stop of a roaming lead: where, how long it takes to get there from the stop before (or, for the
/// first, from where the roaming starts), and how long the lead lingers there.
#[derive(Debug, Clone, PartialEq)]
pub struct Stop {
    pub at: [f64; 3],
    pub travel: f64,
    pub linger: f64,
}

impl Roaming {
    /// The lead point `time` seconds after the roaming starts: easing from stop to stop, still while it
    /// lingers.
    fn at(&self, time: f64) -> Vec3 {
        let mut from = self.stops[0].at;
        let mut t = time;
        for (i, stop) in self.stops.iter().enumerate() {
            if i > 0 {
                if t < stop.travel {
                    let f = t / stop.travel;
                    return lerp(from, stop.at, f * f * (3.0 - 2.0 * f));
                }
                t -= stop.travel;
            }
            if t < stop.linger {
                return stop.at;
            }
            t -= stop.linger;
            from = stop.at;
        }
        from
    }
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
        if let Some(r) = &self.roaming {
            let first = r.at(0.0);
            return if t <= r.start {
                lerp(
                    self.entry,
                    first,
                    (t - self.start) / (r.start - self.start).max(1e-9),
                )
            } else if t <= r.end {
                r.at(t - r.start)
            } else {
                along(r.at(r.end - r.start), self.exit, (t - r.end) * LEAD_SPEED)
            };
        }
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
        random.range(10.0, 18.0),
        random.range(8.0, 24.0),
        side * 2.2,
    );
    let exit = point(
        random.range(10.0, 18.0),
        random.range(8.0, 24.0),
        -side * 2.2,
    );
    // Some big-school visits circle: at one stop the lead lingers 40 to 70 s and the school mills around it.
    let circles = kind == FishKind::Baitfish && random.unit() < 0.4;
    let radius = random.range(6.0, 8.0);
    // Roaming close to the camera: stops around a centre 6 to 9 m ahead and 4 to 10 m above the camera,
    // up to 14 m to either side (past the picture's edges), 3 m up or down and nearer or farther; each 4 to
    // 14 m from the last, reached at about LEAD_SPEED / 1.6 (eased, so never faster than LEAD_SPEED), with
    // 4 to 12 s of lingering.
    let roam_time = match kind {
        FishKind::Baitfish => random.range(70.0, 140.0),
        FishKind::Mackerel => random.range(60.0, 110.0),
    };
    let roam_centre = point(random.range(6.0, 9.0), random.range(20.0, 26.0), 0.0);
    // The stop where a circling school mills: the third.
    const CIRCLING_STOP: usize = 2;
    let mut stops: Vec<Stop> = Vec::new();
    let mut total = 0.0;
    while total < roam_time || (circles && stops.len() <= CIRCLING_STOP) {
        let mut at = roam_centre;
        for _ in 0..8 {
            at = [
                roam_centre[0] + random.range(-14.0, 14.0),
                roam_centre[1] + random.range(-3.0, 3.0),
                roam_centre[2] + random.range(-3.0, 3.0),
            ];
            let hop = stops.last().map_or(8.0, |s| length(sub(at, s.at)));
            if (4.0..=14.0).contains(&hop) {
                break;
            }
        }
        let travel = stops
            .last()
            .map_or(0.0, |s| 1.6 * length(sub(at, s.at)) / LEAD_SPEED);
        let linger = if circles && stops.len() == CIRCLING_STOP {
            random.range(40.0, 70.0)
        } else {
            random.range(4.0, 12.0)
        };
        total += travel + linger;
        stops.push(Stop { at, travel, linger });
    }

    let mut roaming = Roaming {
        start: 0.0,
        end: 0.0,
        stops,
    };
    roaming.start = start + length(sub(roaming.at(0.0), entry)) / LEAD_SPEED;
    roaming.end = roaming.start + total;
    let circling = circles.then(|| {
        let before: f64 = roaming.stops[..CIRCLING_STOP]
            .iter()
            .map(|s| s.travel + s.linger)
            .sum();
        let stop = &roaming.stops[CIRCLING_STOP];
        let begin = roaming.start + before + stop.travel;
        Circling {
            start: begin,
            end: begin + stop.linger,
            centre: stop.at,
            radius,
        }
    });
    let leave = roaming.at(total);
    // A mackerel visit stays under about 210 s and starts at most 60 s into its 300 s slot, so it ends more
    // than 30 s before the next slot's earliest start.
    let end = roaming.end + (length(sub(exit, leave)) + kind.overshoot()) / LEAD_SPEED;
    SchoolVisit {
        id,
        kind,
        count,
        start,
        end,
        entry,
        exit,
        circling,
        roaming: Some(roaming),
    }
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

/// A visit of `kind` under way at `at`, five seconds in, with the schedule's route rules: for the preview's
/// `--school`, to show a school on demand. It is the visit of `at`'s slot moved in time.
pub fn forced_visit(seed: CoveSeed, kind: FishKind, at: f64) -> SchoolVisit {
    let mut visit = visit_in_slot(seed, kind, (at / kind.slot()).floor() as i64);
    let shift = at - 5.0 - visit.start;
    visit.start += shift;
    visit.end += shift;
    if let Some(c) = visit.circling.as_mut() {
        c.start += shift;
        c.end += shift;
    }
    if let Some(r) = visit.roaming.as_mut() {
        r.start += shift;
        r.end += shift;
    }
    visit
}
