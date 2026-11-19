//! The fish schools, simulated on each screen with the zone model of Couzin et al. (2002, "Collective
//! memory and spatial sorting in animal groups", J. theor. Biol. 218) and steered along their shared
//! route by a few informed fish (Couzin et al. 2005, "Effective leadership and decision-making in animal
//! groups on the move", Nature 433).
//!
//! Every screen sees the same visits at the same moments (`cove_scene::schools`), but simulates the fish
//! itself: the school is in the same place everywhere, its shape and its fish differ (ADR 0009).
//!
//! Each fish looks at the others in three zones around it, measured in its body lengths. Too close (the
//! zone of repulsion) it turns away and ignores everything else. Otherwise it turns to swim like the fish of
//! its school in the zone of orientation and towards those in the zone of attraction beyond it, but not at
//! fish in the blind angle behind it. It swims at a constant speed and turns at most so fast (faster when it
//! darts away from a fish too close). A wide zone of orientation gives a parallel group, a narrow one a
//! torus (a mill).
//!
//! Two additions to the papers' model:
//! - Fish prefer to swim level (the vertical part of their wish is damped). Without it a torus tilts and
//!   rarely settles; with it, it mills round the vertical, as the circling route means.
//! - Bodies do not pass through each other: after each step, fish closer than 0.6 of a body length are
//!   moved apart. The turning rules keep them apart almost always; this catches the rest.
//!
//! A circling school mills over the middle of its circle while the route's lead goes round it. Couzin's
//! model has a memory: a parallel group whose zone of orientation narrows stays parallel, and only a
//! disordered one settles into a torus. So the school first swims to the middle (as a parallel group led
//! by its informed fish), loosens there (no orientation zone, much more noise, quicker turns) until it has
//! lost its common heading, and then narrows its orientation zone and mills round the middle, its informed
//! fish swimming round the way the lead goes. If it drifts off instead, it swims back and tries again. It
//! sets off after the lead a while before the circling ends, widening its zone of orientation gradually.
//! The loosening may look abrupt: it is to be tuned by eye in the preview (Task 4).

use cove_scene::scene::FishPose;
use cove_scene::schools::{FishKind, SchoolVisit};
use cove_scene::SplitMix64;
use std::f64::consts::TAU;

type Vec3 = [f64; 3];

/// Neighbours looked at side by side (each cell of the grid holds a whole number of lanes).
const LANES: usize = 8;
/// Where the stand-ins that pad the grid's cells are (metres).
const FAR: f64 = 1.0e6;
/// The fixed integration step, in seconds.
const STEP: f64 = 1.0 / 30.0;
/// A pause longer than this between two calls (sleep, a jump to another moment) restarts the schools.
const GAP: f64 = 2.0;
/// How long a restarted school swims before it is shown, in seconds.
const WARM_UP: f64 = 3.0;
/// A restarted school starts this far apart (in body lengths), its headings scattered by this much
/// (radians).
const SPAWN_SPACING: f64 = 1.5;
const SPAWN_SCATTER: f64 = 0.05;

/// Zone radii in body lengths (Couzin's r_r, r_o and r_a).
const REPULSION: f64 = 1.0;
const ORIENTATION: f64 = 12.0;
const ATTRACTION: f64 = 15.0;
/// The zone of orientation of a milling school: too narrow to hold a parallel group together.
const ORIENTATION_MILLING: f64 = 2.0;
/// A fish perceives 270 degrees around it; this is the cosine of half the blind cone behind it.
const BLIND_COS: f64 = std::f64::consts::FRAC_1_SQRT_2;
/// The fastest turn, in radians per second.
const TURN_RATE: f64 = 3.0;
/// The fastest turn away from a fish too close (a darting escape), in radians per second.
const ESCAPE_TURN_RATE: f64 = 10.0;
/// No two fish end a step closer than this share of the larger one's body length, after at most this many
/// passes of moving them apart (a pass can push a fish into a third).
const CLOSEST: f64 = 0.6;
const SEPARATION_PASSES: usize = 4;
/// The standard deviation of the angular noise, in radians per step.
const NOISE: f64 = 0.03;
/// How much of the wish to climb or dive a fish follows (fish prefer to swim level).
const LEVEL: f64 = 0.25;

/// The share of each school that knows where to go, and how strongly it holds to that (Couzin's omega).
const INFORMED_SHARE: f64 = 0.1;
const OMEGA: f64 = 0.5;
/// Informed fish feel the full pull of their goal from this far off (in metres) and less of it closer in,
/// so that a school that catches up with its lead swings round it instead of turning on the spot.
const PULL_DISTANCE: f64 = 6.0;

/// Circling: the school loosens once its centre is this close to the middle of the circle (metres) ...
const ARRIVED: f64 = 2.0;
/// ... for at least this long (seconds) and until its polarisation is below `LOOSE` (or at most for
/// `LOOSEN_TIME_MAX`), with this noise (radians per step) and turning rate (radians per second).
const LOOSEN_TIME: f64 = 3.0;
const LOOSE: f64 = 0.1;
const LOOSEN_TIME_MAX: f64 = 10.0;
const LOOSEN_NOISE: f64 = 0.6;
const LOOSEN_TURN_RATE: f64 = 6.0;
/// While it mills, its informed fish feel the full pull back to the middle from this far off (metres).
const MILL_PULL_DISTANCE: f64 = 1.0;
/// How strongly the informed fish of a milling school want to swim round its middle (the way the lead
/// goes), next to OMEGA's pull to the middle: enough to tip a disordered school into a mill.
const SPIN: f64 = 1.0;
/// A school whose centre drifts this far from the middle (metres) has not settled into a mill: it swims
/// back and loosens again.
const DRIFTED: f64 = 6.0;
/// ... and widens its zone of orientation back over this long (seconds).
const WIDEN_TIME: f64 = 10.0;
/// It sets off after the lead this long before the circling ends (seconds).
const LEAVE_EARLY: f64 = 15.0;

/// Fish keep this far below the surface (metres, y is up) and outside this sphere around the camera, and
/// start to turn away a metre before.
const SURFACE: f64 = -2.0;
const CAMERA_CLEARANCE: f64 = 3.0;
const CAMERA: Vec3 = [0.0, -30.0, 0.0];
const MARGIN: f64 = 1.0;

fn speed(kind: FishKind) -> f64 {
    match kind {
        FishKind::Baitfish => 1.1,
        FishKind::Mackerel => 1.3,
    }
}

/// The mean body length in metres; each fish is up to 10 % shorter or longer.
fn body_length(kind: FishKind) -> f64 {
    match kind {
        FishKind::Baitfish => 0.18,
        FishKind::Mackerel => 0.35,
    }
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: Vec3, s: f64) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}

/// The unit vector along `a`, or `None` if it is (nearly) zero.
fn unit(a: Vec3) -> Option<Vec3> {
    let l = length(a);
    (l > 1e-12).then(|| scale(a, 1.0 / l))
}

/// A standard normal value (Box-Muller).
fn gaussian(random: &mut SplitMix64) -> f64 {
    let u = random.unit().max(1e-300);
    (-2.0 * u.ln()).sqrt() * (TAU * random.unit()).cos()
}

/// `heading` turned towards `wanted` by at most `angle` radians (both unit vectors).
fn turn_towards(heading: Vec3, wanted: Vec3, angle: f64) -> Vec3 {
    let c = dot(heading, wanted).clamp(-1.0, 1.0);
    if c.acos() <= angle {
        return wanted;
    }
    // The direction to turn in, square to the heading; any will do if `wanted` is straight behind.
    let side = unit(sub(wanted, scale(heading, c)))
        .or_else(|| unit([-heading[2], 0.0, heading[0]]))
        .unwrap_or([0.0, 1.0, 0.0]);
    add(scale(heading, angle.cos()), scale(side, angle.sin()))
}

struct Fish {
    position: Vec3,
    heading: Vec3,
    size: f64,
    tail_phase: f64,
    /// Index into `Swarm::schools`.
    school: usize,
    informed: bool,
}

struct School {
    visit: SchoolVisit,
    /// When it was put in the water.
    spawned: f64,
    /// Where it is in its circling.
    phase: Phase,
}

/// The steps of a circling school.
#[derive(Clone, Copy, PartialEq)]
enum Phase {
    /// On its way to the middle of the circle (and on its route when not circling).
    Gather,
    /// Losing its common heading, since this moment.
    Loosen(f64),
    Mill,
}

/// What a school does during one step.
struct Behaviour {
    /// The zone of orientation, in body lengths.
    orientation: f64,
    /// Where its informed fish want to go, and from how far off (metres) they feel its full pull.
    goal: Vec3,
    pull: f64,
    /// While milling, the way round the goal the informed fish also want to swim (+1 or -1, 0 for none).
    spin: f64,
    noise: f64,
    turn_rate: f64,
}

impl School {
    fn behaviour(&self, time: f64) -> Behaviour {
        let travel = Behaviour {
            orientation: ORIENTATION,
            goal: self.visit.lead(time),
            pull: PULL_DISTANCE,
            spin: 0.0,
            noise: NOISE,
            turn_rate: TURN_RATE,
        };
        let Some(c) = self.visit.circling.filter(|_| self.circles(time)) else {
            // Setting off from the mill: the zone of orientation widens gradually, so that the informed
            // fish turn the school towards the lead before it lines up in some other direction.
            if let Some(c) = self.visit.circling {
                let since = time - (c.end - LEAVE_EARLY);
                if (0.0..LEAVE_EARLY).contains(&since) {
                    let share = (since / WIDEN_TIME).min(1.0);
                    return Behaviour {
                        orientation: ORIENTATION_MILLING
                            + (ORIENTATION - ORIENTATION_MILLING) * share,
                        ..travel
                    };
                }
            }
            return travel;
        };
        match self.phase {
            // Swim to the middle as a parallel group.
            Phase::Gather => Behaviour {
                goal: c.centre,
                ..travel
            },
            // Loosen.
            Phase::Loosen(_) => Behaviour {
                orientation: REPULSION,
                goal: c.centre,
                pull: MILL_PULL_DISTANCE,
                spin: 0.0,
                noise: LOOSEN_NOISE,
                turn_rate: LOOSEN_TURN_RATE,
            },
            // Mill.
            Phase::Mill => Behaviour {
                orientation: ORIENTATION_MILLING,
                goal: c.centre,
                pull: MILL_PULL_DISTANCE,
                // The way the lead goes round (see `Circling` in cove-scene).
                spin: if self.visit.entry[0] < 0.0 { 1.0 } else { -1.0 },
                ..travel
            },
        }
    }

    /// Whether the school keeps to its circle at `time` (it sets off a little before the lead does).
    fn circles(&self, time: f64) -> bool {
        self.visit
            .circling
            .is_some_and(|c| c.start <= time && time < c.end - LEAVE_EARLY)
    }
}

/// The fish sorted by grid cell, with their data side by side for the neighbour search.
#[derive(Default)]
struct Grid {
    /// Packed cell key, start and end index of each occupied cell, sorted by key.
    cells: Vec<(u64, usize, usize)>,
    /// Fish indices in cell order (`usize::MAX` for the stand-ins that pad each cell to whole lanes).
    order: Vec<usize>,
    position: Vec<Vec3>,
    /// The neighbours' data in single precision, a component per array, for a loop without branches.
    x: Vec<f32>,
    y: Vec<f32>,
    z: Vec<f32>,
    hx: Vec<f32>,
    hy: Vec<f32>,
    hz: Vec<f32>,
    size: Vec<f32>,
    school: Vec<f32>,
}

fn cell_of(p: Vec3, cell: f64) -> [i64; 3] {
    p.map(|c| (c / cell).floor() as i64)
}

fn pack(c: [i64; 3]) -> u64 {
    const BIAS: i64 = 1 << 20;
    let part = |v: i64| ((v + BIAS).clamp(0, (1 << 21) - 1)) as u64;
    (part(c[0]) << 42) | (part(c[1]) << 21) | part(c[2])
}

impl Grid {
    fn build(&mut self, fish: &[Fish], cell: f64) {
        let mut keyed: Vec<(u64, usize)> = fish
            .iter()
            .enumerate()
            .map(|(i, f)| (pack(cell_of(f.position, cell)), i))
            .collect();
        keyed.sort_unstable();
        self.cells.clear();
        self.order.clear();
        self.position.clear();
        for v in [
            &mut self.x,
            &mut self.y,
            &mut self.z,
            &mut self.hx,
            &mut self.hy,
            &mut self.hz,
            &mut self.size,
            &mut self.school,
        ] {
            v.clear();
        }
        for &(key, i) in &keyed {
            if self.cells.last().is_none_or(|last| last.0 != key) {
                self.pad();
                let n = self.order.len();
                self.cells.push((key, n, n));
            }
            let f = &fish[i];
            self.order.push(i);
            self.position.push(f.position);
            self.x.push(f.position[0] as f32);
            self.y.push(f.position[1] as f32);
            self.z.push(f.position[2] as f32);
            self.hx.push(f.heading[0] as f32);
            self.hy.push(f.heading[1] as f32);
            self.hz.push(f.heading[2] as f32);
            self.size.push(f.size as f32);
            self.school.push(f.school as f32);
        }
        self.pad();
    }

    /// Fills the last cell up to a whole number of lanes with stand-ins far away, which no fish sees.
    fn pad(&mut self) {
        while !self.order.len().is_multiple_of(LANES) {
            self.order.push(usize::MAX);
            self.position.push([FAR; 3]);
            for v in [&mut self.x, &mut self.y, &mut self.z] {
                v.push(FAR as f32);
            }
            for v in [&mut self.hx, &mut self.hy, &mut self.hz, &mut self.size] {
                v.push(0.0);
            }
            self.school.push(-1.0);
        }
        if let Some(last) = self.cells.last_mut() {
            last.2 = self.order.len();
        }
    }

    /// The index ranges of the occupied cells around (and including) `centre`.
    fn around(&self, centre: [i64; 3], ranges: &mut Vec<(usize, usize)>) {
        ranges.clear();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let key = pack([centre[0] + dx, centre[1] + dy, centre[2] + dz]);
                    if let Ok(k) = self.cells.binary_search_by_key(&key, |c| c.0) {
                        ranges.push((self.cells[k].1, self.cells[k].2));
                    }
                }
            }
        }
    }
}

pub struct Swarm {
    fish: Vec<Fish>,
    schools: Vec<School>,
    random: SplitMix64,
    /// The moment of the last call, and the simulated time (behind it by less than a step).
    last: Option<f64>,
    clock: f64,
    /// Kept between steps for its allocations.
    grid: Grid,
}

impl Swarm {
    pub fn new(seed: u64) -> Self {
        Self {
            fish: Vec::new(),
            schools: Vec::new(),
            random: SplitMix64::new(seed),
            last: None,
            clock: 0.0,
            grid: Grid::default(),
        }
    }

    /// Brings the swarm to `at` for the visits under way: spawns new visits' schools, removes ended ones,
    /// and integrates. A first call, a jump back in time or a gap over 0.25 s restarts every school
    /// mid-visit (spawned as a loose parallel group around `lead(at)`, heading along the route, then 3 s
    /// of warm-up).
    pub fn advance(&mut self, visits: &[SchoolVisit], at: f64) {
        let restart = match self.last {
            None => true,
            Some(last) => at < last || at - last > GAP,
        };
        if restart {
            self.fish.clear();
            self.schools.clear();
            // Spawned where the lead was a warm-up ago, the school is near it again at `at`.
            self.clock = at - WARM_UP;
        } else {
            self.remove_ended(visits);
        }
        for visit in visits {
            if !self.schools.iter().any(|s| s.visit.id == visit.id) {
                self.spawn(visit, self.clock);
            }
        }
        self.last = Some(at);
        while self.clock + STEP <= at + 1e-9 {
            self.clock += STEP;
            self.step(self.clock);
        }
    }

    pub fn poses(&self) -> Vec<FishPose> {
        self.fish
            .iter()
            .map(|f| {
                let kind = self.schools[f.school].visit.kind;
                FishPose {
                    position: f.position,
                    heading: f.heading,
                    kind,
                    size: f.size,
                    tail_phase: f.tail_phase,
                    speed: speed(kind),
                }
            })
            .collect()
    }

    fn remove_ended(&mut self, visits: &[SchoolVisit]) {
        let kept: Vec<bool> = self
            .schools
            .iter()
            .map(|s| visits.iter().any(|v| v.id == s.visit.id))
            .collect();
        if kept.iter().all(|&k| k) {
            return;
        }
        // The new index of each kept school.
        let mut index = Vec::with_capacity(kept.len());
        let mut next = 0;
        for &k in &kept {
            index.push(next);
            next += usize::from(k);
        }
        self.fish.retain(|f| kept[f.school]);
        for f in &mut self.fish {
            f.school = index[f.school];
        }
        let mut k = kept.iter();
        self.schools.retain(|_| *k.next().unwrap_or(&false));
    }

    /// Puts a visit's school in the water as a loose parallel group around the lead at `time`.
    fn spawn(&mut self, visit: &SchoolVisit, time: f64) {
        let school = self.schools.len();
        self.schools.push(School {
            visit: visit.clone(),
            spawned: time,
            phase: Phase::Gather,
        });
        let centre = visit.lead(time);
        let ahead = unit(sub(visit.lead(time + 1.0), visit.lead(time - 1.0)))
            .or_else(|| unit(sub(visit.exit, visit.entry)))
            .unwrap_or([1.0, 0.0, 0.0]);
        let body = body_length(visit.kind);
        // A ball cut from a jittered lattice, a little looser than the school swims, so that no two fish
        // start too close and the school need not crowd in from afar.
        let spacing = SPAWN_SPACING * body;
        let reach = ((visit.count as f64).cbrt() * 0.75).ceil() as i32 + 3;
        let mut places = Vec::new();
        for x in -reach..=reach {
            for y in -reach..=reach {
                for z in -reach..=reach {
                    let jitter = [(); 3].map(|_| self.random.range(-0.15, 0.15));
                    places.push(scale(add([x as f64, y as f64, z as f64], jitter), spacing));
                }
            }
        }
        // Not on top of another school's fish.
        let clear = SPAWN_SPACING * body;
        places.retain(|&p| {
            let at = add(centre, p);
            self.fish
                .iter()
                .all(|f| length(sub(f.position, at)) >= clear.max(SPAWN_SPACING * f.size))
        });
        places.sort_by(|a, b| length(*a).total_cmp(&length(*b)));
        places.truncate(visit.count);
        // Shuffled, so the informed fish (the first few) are anywhere in the school.
        for i in (1..places.len()).rev() {
            places.swap(i, self.random.int(0, i as i64) as usize);
        }
        let informed = (visit.count as f64 * INFORMED_SHARE).round() as usize;
        for (n, offset) in places.into_iter().enumerate() {
            let mut position = add(centre, offset);
            position[1] = position[1].min(SURFACE - MARGIN);
            let scatter = [(); 3].map(|_| SPAWN_SCATTER * gaussian(&mut self.random));
            self.fish.push(Fish {
                position,
                heading: unit(add(ahead, scatter)).unwrap_or(ahead),
                size: body * self.random.range(0.9, 1.1),
                tail_phase: self.random.angle(),
                school,
                informed: n < informed,
            });
        }
    }

    /// Notes when a circling school reaches the middle of its circle (or should have), sends it back there
    /// if it has drifted off instead of milling, and forgets it all once the circling is over.
    fn track_circling(&mut self, time: f64) {
        for (index, school) in self.schools.iter_mut().enumerate() {
            let Some(c) = school.visit.circling else {
                continue;
            };
            if !school.circles(time) {
                school.phase = Phase::Gather;
                continue;
            }
            let (mut sum, mut heading, mut n) = ([0.0; 3], [0.0; 3], 0.0);
            for f in self.fish.iter().filter(|f| f.school == index) {
                sum = add(sum, f.position);
                heading = add(heading, f.heading);
                n += 1.0;
            }
            if n == 0.0 {
                continue;
            }
            let off = length(sub(scale(sum, 1.0 / n), c.centre));
            let polarisation = length(heading) / n;
            school.phase = match school.phase {
                Phase::Gather => {
                    // However the school came, it has had time to get there.
                    let late = time - c.start.max(school.spawned)
                        > 2.0 * c.radius / speed(school.visit.kind);
                    if off < ARRIVED || late {
                        Phase::Loosen(time)
                    } else {
                        Phase::Gather
                    }
                }
                Phase::Loosen(since) => {
                    let loose = time - since >= LOOSEN_TIME && polarisation < LOOSE;
                    if loose || time - since >= LOOSEN_TIME_MAX {
                        Phase::Mill
                    } else {
                        Phase::Loosen(since)
                    }
                }
                Phase::Mill if off > DRIFTED => Phase::Gather,
                Phase::Mill => Phase::Mill,
            };
        }
    }

    /// One step of the zone model at `time`: every fish decides from the same snapshot, then all move.
    fn step(&mut self, time: f64) {
        if self.fish.is_empty() {
            return;
        }
        self.track_circling(time);
        let behaviours: Vec<Behaviour> = self.schools.iter().map(|s| s.behaviour(time)).collect();

        // A uniform grid with cells as wide as the widest zone of attraction: every fish a fish can see
        // is in its own cell or one of the 26 around it.
        let cell = self
            .fish
            .iter()
            .map(|f| f.size * ATTRACTION)
            .fold(0.0, f64::max);
        self.grid.build(&self.fish, cell);

        let mut wanted = vec![([0.0; 3], false); self.fish.len()];
        let mut ranges = Vec::with_capacity(27);
        for &(_, from, to) in &self.grid.cells {
            self.grid
                .around(cell_of(self.grid.position[from], cell), &mut ranges);
            for &index in self.grid.order[from..to]
                .iter()
                .filter(|&&i| i != usize::MAX)
            {
                let me = &self.fish[index];
                wanted[index] = self.decide(me, &ranges, &behaviours[me.school]);
            }
        }

        for (f, (direction, repelled)) in self.fish.iter_mut().zip(wanted) {
            let behaviour = &behaviours[f.school];
            let noise = [(); 3].map(|_| behaviour.noise * gaussian(&mut self.random));
            let across = sub(noise, scale(direction, dot(noise, direction)));
            let direction = unit(add(direction, across)).unwrap_or(direction);
            f.heading = unit(turn_towards(
                f.heading,
                direction,
                if repelled {
                    behaviour.turn_rate.max(ESCAPE_TURN_RATE)
                } else {
                    behaviour.turn_rate
                } * STEP,
            ))
            .unwrap_or(f.heading);
            let v = speed(self.schools[f.school].visit.kind);
            f.position = add(f.position, scale(f.heading, v * STEP));
            f.tail_phase = (f.tail_phase + TAU * v / f.size * STEP) % TAU;
        }
        self.keep_apart();
    }

    /// Bodies do not pass through each other: where the turning rules (a turn rate, a constant speed) leave
    /// two fish closer than `CLOSEST` of the larger one's length (in a loosening burst, say), both are
    /// moved apart along the line between them. Only positions change; headings stay the model's.
    fn keep_apart(&mut self) {
        let cell = self.fish.iter().map(|f| f.size).fold(0.0, f64::max) * CLOSEST;
        if cell <= 0.0 {
            return;
        }
        // Each pair once: a cell and the 13 of its neighbours that come after it.
        let mut forward = Vec::with_capacity(14);
        for dx in -1..=1i64 {
            for dy in -1..=1i64 {
                for dz in -1..=1i64 {
                    if (dx, dy, dz) >= (0, 0, 0) {
                        forward.push([dx, dy, dz]);
                    }
                }
            }
        }
        let mut keyed: Vec<(u64, usize)> = Vec::with_capacity(self.fish.len());
        let mut cells: Vec<(u64, usize, usize)> = Vec::new();
        for _ in 0..SEPARATION_PASSES {
            keyed.clear();
            keyed.extend(
                self.fish
                    .iter()
                    .enumerate()
                    .map(|(i, f)| (pack(cell_of(f.position, cell)), i)),
            );
            keyed.sort_unstable();
            cells.clear();
            for (n, &(key, _)) in keyed.iter().enumerate() {
                match cells.last_mut() {
                    Some(last) if last.0 == key => last.2 = n + 1,
                    _ => cells.push((key, n, n + 1)),
                }
            }
            let mut moved = false;
            for &(_, from, to) in &cells {
                let home = cell_of(self.fish[keyed[from].1].position, cell);
                for offset in &forward {
                    let key = pack([
                        home[0] + offset[0],
                        home[1] + offset[1],
                        home[2] + offset[2],
                    ]);
                    let Ok(k) = cells.binary_search_by_key(&key, |c| c.0) else {
                        continue;
                    };
                    let (start, end) = (cells[k].1, cells[k].2);
                    let same = *offset == [0, 0, 0];
                    for a in from..to {
                        for b in if same { a + 1 } else { start }..end {
                            let (i, j) = (keyed[a].1, keyed[b].1);
                            let least = self.fish[i].size.max(self.fish[j].size) * CLOSEST;
                            let d = sub(self.fish[j].position, self.fish[i].position);
                            let squared = dot(d, d);
                            if squared >= least * least {
                                continue;
                            }
                            let distance = squared.sqrt();
                            let along = unit(d).unwrap_or(self.fish[i].heading);
                            let push = scale(along, 0.5 * (least - distance));
                            self.fish[i].position = sub(self.fish[i].position, push);
                            self.fish[j].position = add(self.fish[j].position, push);
                            moved = true;
                        }
                    }
                }
            }
            if !moved {
                break;
            }
        }
    }

    /// The direction a fish wants to swim in, and whether it is turning away from something too close.
    fn decide(&self, me: &Fish, ranges: &[(usize, usize)], b: &Behaviour) -> (Vec3, bool) {
        let grid = &self.grid;
        let (px, py, pz) = (
            me.position[0] as f32,
            me.position[1] as f32,
            me.position[2] as f32,
        );
        let (hx, hy, hz) = (
            me.heading[0] as f32,
            me.heading[1] as f32,
            me.heading[2] as f32,
        );
        let size = me.size as f32;
        let reach = size * ATTRACTION as f32;
        let orientation = size * b.orientation as f32;
        let school = me.school as f32;
        let blind = (BLIND_COS * BLIND_COS) as f32;
        // Sums of: the unit vectors to fish too close, the headings of fish to align with, and the unit
        // vectors to fish to swim towards; and their counts. Each zone is a 0-or-1 factor, not a branch.
        let mut lanes = [[0.0f32; LANES]; 12];
        for &(from, to) in ranges {
            let n = to - from;
            let (xs, ys, zs) = (&grid.x[from..to], &grid.y[from..to], &grid.z[from..to]);
            let (hxs, hys, hzs) = (&grid.hx[from..to], &grid.hy[from..to], &grid.hz[from..to]);
            let (sizes, schools) = (&grid.size[from..to], &grid.school[from..to]);
            let (xs, ys, zs, hxs, hys, hzs, sizes, schools) = (
                &xs[..n],
                &ys[..n],
                &zs[..n],
                &hxs[..n],
                &hys[..n],
                &hzs[..n],
                &sizes[..n],
                &schools[..n],
            );
            // Several neighbours at a time, in lanes, so that the compiler can use vector instructions.
            for start in (0..n).step_by(LANES) {
                let lane = |v: &[f32]| -> [f32; LANES] {
                    v[start..start + LANES].try_into().unwrap_or([0.0; LANES])
                };
                let (x, y, z) = (lane(xs), lane(ys), lane(zs));
                let (ox, oy, oz) = (lane(hxs), lane(hys), lane(hzs));
                let (sz, sc) = (lane(sizes), lane(schools));
                let mut terms = [[0.0f32; LANES]; 12];
                for l in 0..LANES {
                    let (dx, dy, dz) = (x[l] - px, y[l] - py, z[l] - pz);
                    let squared = dx * dx + dy * dy + dz * dz;
                    let inverse = 1.0 / squared.max(1e-12).sqrt();
                    let other = squared > 1e-12;
                    let repulsion = size.max(sz[l]) * REPULSION as f32;
                    let near = other & (squared < repulsion * repulsion);
                    let ahead = dx * hx + dy * hy + dz * hz;
                    let seen = (ahead >= 0.0) | (ahead * ahead <= blind * squared);
                    let social = other & !near & seen & (sc[l] == school);
                    let inside = squared < orientation * orientation;
                    let align = f32::from(u8::from(social & inside));
                    let attract = f32::from(u8::from(social & !inside & (squared < reach * reach)));
                    let repel = f32::from(u8::from(near));
                    let push = repel * inverse;
                    let pull = attract * inverse;
                    terms[0][l] = -dx * push;
                    terms[1][l] = -dy * push;
                    terms[2][l] = -dz * push;
                    terms[3][l] = repel;
                    terms[4][l] = ox[l] * align;
                    terms[5][l] = oy[l] * align;
                    terms[6][l] = oz[l] * align;
                    terms[7][l] = align;
                    terms[8][l] = dx * pull;
                    terms[9][l] = dy * pull;
                    terms[10][l] = dz * pull;
                    terms[11][l] = attract;
                }
                for (sum, term) in lanes.iter_mut().zip(&terms) {
                    for l in 0..LANES {
                        sum[l] += term[l];
                    }
                }
            }
        }
        let mut sums = [0.0f32; 12];
        for (sum, lane) in sums.iter_mut().zip(&lanes) {
            *sum = lane.iter().sum();
        }
        let vector = |k: usize| [sums[k] as f64, sums[k + 1] as f64, sums[k + 2] as f64];
        let mut away = vector(0);
        let mut repelled = sums[3] > 0.0;
        let (aligned, attracted) = (sums[7] > 0.0, sums[11] > 0.0);
        // The fish's own heading counts in the zone of orientation (Couzin et al. 2002).
        let align = add(vector(4), me.heading);
        let towards = vector(8);

        // The surface and the camera push like a neighbour too close.
        if me.position[1] > SURFACE - MARGIN {
            away = add(away, [0.0, -1.0, 0.0]);
            repelled = true;
        }
        let from_camera = sub(me.position, CAMERA);
        if length(from_camera) < CAMERA_CLEARANCE + MARGIN {
            away = add(away, unit(from_camera).unwrap_or([0.0, 1.0, 0.0]));
            repelled = true;
        }
        if repelled {
            return (unit(away).unwrap_or(me.heading), true);
        }

        let align = aligned.then(|| unit(align)).flatten();
        let towards = attracted.then(|| unit(towards)).flatten();
        let mut direction = match (align, towards) {
            (Some(a), Some(t)) => unit(add(a, t)).unwrap_or(me.heading),
            (Some(a), None) => a,
            (None, Some(t)) => t,
            (None, None) => me.heading,
        };
        if me.informed {
            let to_goal = sub(b.goal, me.position);
            let d = length(to_goal).max(1e-9);
            let weight = OMEGA * (d / b.pull).min(1.0);
            direction = add(direction, scale(to_goal, weight / d));
            let (rx, rz) = (-to_goal[0], -to_goal[2]);
            let r = rx.hypot(rz);
            if b.spin != 0.0 && r > 1e-6 {
                let round = [-b.spin * rz / r, 0.0, b.spin * rx / r];
                direction = add(direction, scale(round, SPIN));
            }
        }
        direction[1] *= LEVEL;
        (unit(direction).unwrap_or(me.heading), false)
    }
}

/// How alike the fish swim: the length of their mean heading, 1 for a parallel group, near 0 for a swarm
/// or a mill.
pub fn polarisation(fish: &[FishPose]) -> f64 {
    if fish.is_empty() {
        return 0.0;
    }
    let sum = fish.iter().fold([0.0; 3], |s, f| add(s, f.heading));
    length(sum) / fish.len() as f64
}

/// How much the fish swim round their centre (about the vertical): the mean of (r x h).y over the fish,
/// with r the unit vector from the centre to the fish and h its heading; 1 for a perfect mill.
pub fn milling(fish: &[FishPose]) -> f64 {
    if fish.is_empty() {
        return 0.0;
    }
    let n = fish.len() as f64;
    let centre = scale(
        fish.iter().fold([0.0; 3], |s, f| add(s, f.position)),
        1.0 / n,
    );
    let sum: f64 = fish
        .iter()
        .filter_map(|f| {
            let r = unit(sub(f.position, centre))?;
            Some(r[2] * f.heading[0] - r[0] * f.heading[2])
        })
        .sum();
    (sum / n).abs()
}
