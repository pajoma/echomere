//! A cove's scene. Port of `CoveScene.swift`.
//!
//! Every screen in the cove computes the same frame from the same seed, visitors, echoes and time, so they
//! all show the same picture without anything being streamed (ADR 0005).

use crate::camera;
use crate::kelp::{self, cycle, wrap_angle, KelpBlade, KelpMotion};
use crate::layout::CoveLayout;
use crate::model::{scene_rules, CoveSeed, Echo, Visitor, WhaleLook, WhaleSpecies};
use crate::random::{combine, stable_hash, SplitMix64};
use crate::schools::{self, FishKind, SchoolVisit};
use crate::sea;
use crate::sky::{self, SkyState};
use std::cmp::Ordering;
use std::f64::consts::PI;

/// One fish of a school, in metres: x to the right of the camera, y up (0 is the surface), z ahead of it.
/// `CoveScene::frame` leaves these empty; whoever simulates the schools fills them in.
#[derive(Debug, Clone, PartialEq)]
pub struct FishPose {
    pub position: [f64; 3],
    /// The way the fish swims (a unit vector).
    pub heading: [f64; 3],
    pub kind: FishKind,
    /// Length in metres.
    pub size: f64,
    /// Tail beat, 0..2pi.
    pub tail_phase: f64,
    /// Metres per second.
    pub speed: f64,
}

/// A whale in the water, in metres: x to the right of the camera, y up (0 is the surface), z ahead of it.
#[derive(Debug, Clone, PartialEq)]
pub struct WhalePose {
    pub id: String,
    pub species: WhaleSpecies,
    pub position: [f64; 3],
    /// The way the whale swims (a unit vector).
    pub forward: [f64; 3],
    /// How far it banks into a turn, in radians.
    pub roll: f64,
    /// Tail beat, 0..2pi.
    pub tail_phase: f64,
    /// Length in metres.
    pub length: f64,
    /// How hard it swims: 1 beats the tail fully, near 0 it glides.
    pub effort: f64,
    pub opacity: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EchoPose {
    pub x: f64,
    pub y: f64,
    pub depth: f64,
    /// 1 when fresh, fading to 0 over `ECHO_LIFETIME`.
    pub intensity: f64,
    /// Progress 0..1 of a distant call (an expanding ring), or `None` when the echo is silent.
    pub call: Option<f64>,
    pub size: f64,
}

/// Everything on screen at one moment.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneFrame {
    /// Seconds since 1970.
    pub time: f64,
    pub whales: Vec<WhalePose>,
    /// The school visits under way, ordered by start.
    pub visits: Vec<SchoolVisit>,
    /// The fish of those visits. Empty from `CoveScene::frame`; the caller fills it in.
    pub fish: Vec<FishPose>,
    pub kelp: Vec<KelpBlade>,
    pub kelp_motion: KelpMotion,
    pub echoes: Vec<EchoPose>,
    /// The time within the sea's loop, in seconds.
    pub sea_time: f64,
    /// The sky above the cove. It can be replaced to look at another time of day.
    pub sky: SkyState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoveScene {
    pub seed: CoveSeed,
    pub layout: CoveLayout,
}

fn cmp_f(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

impl CoveScene {
    pub fn new(seed: CoveSeed) -> Self {
        Self {
            seed,
            layout: CoveLayout::new(seed),
        }
    }

    pub fn frame(&self, at: f64, visitors: &[Visitor], echoes: &[Echo]) -> SceneFrame {
        let t = at;

        let mut present: Vec<&Visitor> = visitors.iter().filter(|v| v.arrived_at <= at).collect();
        present.sort_by(|a, b| {
            cmp_f(a.arrived_at, b.arrived_at).then_with(|| a.creature_id.cmp(&b.creature_id))
        });
        let whales: Vec<WhalePose> = present
            .into_iter()
            .take(scene_rules::MAX_WHALES)
            .map(|v| self.whale_pose(v, t))
            .collect();

        let mut recent: Vec<&Echo> = echoes
            .iter()
            .filter(|e| e.left_at <= at && at - e.left_at < scene_rules::ECHO_LIFETIME)
            .collect();
        recent.sort_by(|a, b| {
            cmp_f(b.left_at, a.left_at).then_with(|| b.creature_id.cmp(&a.creature_id))
        });
        let echoes: Vec<EchoPose> = recent
            .into_iter()
            .take(scene_rules::MAX_ECHOES)
            .map(|e| self.echo_pose(e, t))
            .collect();

        SceneFrame {
            time: at,
            whales,
            visits: schools::visits(self.seed, at),
            fish: Vec::new(),
            kelp: self.layout.kelp.clone(),
            kelp_motion: kelp::motion(at),
            echoes,
            sea_time: sea::time(at),
            sky: sky::state_at(at),
        }
    }

    fn whale_pose(&self, visitor: &Visitor, t: f64) -> WhalePose {
        let path = WhalePath::new(self.seed, &visitor.creature_id, visitor.look);
        let v = path.velocity(t);
        let ground = (v[0] * v[0] + v[2] * v[2]).sqrt();
        let speed = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let heading = path.heading(t);
        // A whale tilts up when it climbs and down when it dives, but never steeply.
        let pitch = v[1].atan2(ground.max(0.45)).max(-0.5).min(0.5);
        // It banks into its turns, more the faster it turns, and rolls a little, lazily, as it swims.
        let mut turn = path.heading(t + 1.0) - path.heading(t - 1.0);
        if turn > PI {
            turn -= 2.0 * PI;
        }
        if turn < -PI {
            turn += 2.0 * PI;
        }
        let bank = (turn / 2.0 * 4.0).max(-0.3).min(0.3) * sky::smooth(0.15, 0.4, ground);
        let since_arrival = t - visitor.arrived_at;
        WhalePose {
            id: visitor.creature_id.clone(),
            species: visitor.look.species,
            position: path.position(t),
            forward: [
                heading.sin() * pitch.cos(),
                pitch.sin(),
                heading.cos() * pitch.cos(),
            ],
            roll: bank + 0.12 * (cycle(t, path.roll_period) + path.roll_phase).sin(),
            tail_phase: wrap_angle(cycle(t, path.tail_period) + path.tail_phase),
            length: visitor.look.length(),
            effort: (speed / 0.8).max(0.15).min(1.0),
            opacity: (since_arrival / scene_rules::WHALE_FADE_IN)
                .max(0.0)
                .min(1.0),
        }
    }

    fn echo_pose(&self, echo: &Echo, t: f64) -> EchoPose {
        let left = echo.left_at;
        // The echo stays where the whale swam in this cove when it left, as the camera saw it.
        let place = WhalePath::new(self.seed, &echo.creature_id, echo.look).position(left);
        let seen = camera::project(place, scene_rules::WORLD_WIDTH);
        let (sx, sy, sd) = match seen {
            Some((x, y, d)) => (Some(x), Some(y), Some(d)),
            None => (None, None, None),
        };
        let pos_x =
            scene_rules::WORLD_CENTRE_X + (sx.unwrap_or(0.5) - 0.5) * scene_rules::WORLD_WIDTH;
        let pos_y = sy.unwrap_or(0.5).max(0.15).min(0.85);
        let pos_depth = (sd.unwrap_or(40.0) / 90.0).max(0.1).min(0.9);
        let mut random = SplitMix64::new(combine(
            self.seed.0 ^ 0xEC40,
            stable_hash(&echo.creature_id),
        ));
        let call_period = random.range(900.0, 2400.0);
        let call_offset = random.range(0.0, call_period);
        let call_duration = 8.0;
        let since_call = (t - left + call_offset) % call_period;
        // Right whales call most at dusk and at night: then the echo calls twice as often.
        let night = echo.look.species == WhaleSpecies::SouthernRightWhale
            && sky::sun_elevation(sky::hour(t)) < 4.0;
        let second_call = (t - left + call_offset + call_period / 2.0) % call_period;
        let width = scene_rules::WORLD_WIDTH;
        EchoPose {
            x: pos_x.max(0.1 * width).min(0.9 * width),
            y: pos_y,
            depth: pos_depth,
            intensity: (1.0 - (t - left) / scene_rules::ECHO_LIFETIME).max(0.0),
            call: if since_call < call_duration {
                Some(since_call / call_duration)
            } else if night && second_call < call_duration {
                Some(second_call / call_duration)
            } else {
                None
            },
            size: echo.look.size,
        }
    }
}

/// Where the whale or its pod goes, and when it breathes.
struct Route {
    seed: u64,
    amplitude_x: f64,
    period_x: f64,
    phase_x: f64,
    centre_z: f64,
    amplitude_z: f64,
    phase_z: f64,
    period_y: f64,
    phase_y: f64,
    /// The whale comes to the front once in this many rounds of its route.
    rounds_per_visit: f64,
    nearest: f64,
    breathing_cycle: f64,
    breathing_offset: f64,
    breath_interval: f64,
    breaths: i64,
}

impl Route {
    fn new(seed: u64, species: WhaleSpecies) -> Self {
        let mut random = SplitMix64::new(seed);
        let amplitude_x = random.range(95.0, 125.0);
        let period_x = random.range(520.0, 760.0) / species.pace();
        let phase_x = random.angle();
        let centre_z = random.range(62.0, 76.0);
        let amplitude_z = random.range(14.0, 20.0);
        // Kept away from a quarter turn, where the figure of eight would have a stopping point.
        let phase_z = random.range(-0.9, 0.9);
        let period_y = random.range(240.0, 420.0);
        let phase_y = random.angle();
        let rounds_per_visit = (random.range(3600.0, 7200.0) / period_x).round().max(3.0);
        let nearest = random.range(14.0, 22.0);
        let (cycle_lo, cycle_hi) = species.breathing_cycle();
        let breathing_cycle = random.range(cycle_lo, cycle_hi);
        let breathing_offset = random.range(0.0, breathing_cycle);
        let (breaths_lo, breaths_hi) = species.breaths();
        let breaths = random.int(breaths_lo, breaths_hi);
        let breath_interval = species.breath_interval() * random.range(0.9, 1.1);
        Self {
            seed,
            amplitude_x,
            period_x,
            phase_x,
            centre_z,
            amplitude_z,
            phase_z,
            period_y,
            phase_y,
            rounds_per_visit,
            nearest,
            breathing_cycle,
            breathing_offset,
            breath_interval,
            breaths,
        }
    }

    /// Where the route is over the ground (x, z) at a moment. The route is a wide figure of eight, traced by
    /// one clock, so the whale never comes to a stop and always has a direction.
    fn ground(&self, t: f64) -> [f64; 2] {
        let around = cycle(t, self.period_x) + self.phase_x;
        // The visit to the front: 0 for most of the time, rising to 1 for a short while.
        let mut long =
            cycle(t, self.period_x * self.rounds_per_visit) + self.phase_x / self.rounds_per_visit;
        long %= 2.0 * PI;
        if long > PI {
            long -= 2.0 * PI;
        }
        let from_crossing = long * self.rounds_per_visit; // in radians of the route
        let visit = (-(from_crossing / 0.55) * (from_crossing / 0.55)).exp();
        let far = self.centre_z + self.amplitude_z * (2.0 * around + self.phase_z).sin();
        [
            self.amplitude_x * (1.0 - 0.6 * visit) * around.sin(),
            far + (self.nearest - far) * visit,
        ]
    }
}

struct Breathing {
    at_surface: f64,
    surface_y: f64,
    dive_y: f64,
}

struct WhalePath {
    species: WhaleSpecies,
    route: Route,
    /// An orca's place in its pod: seconds behind the lead, metres from the pod's line, metres above or
    /// below, and seconds after the pod's breath. All zero for a right whale.
    behind: f64,
    beside: f64,
    beside_angle: f64,
    above: f64,
    breath_delay: f64,
    wander_period: f64,
    wander_phase: f64,
    /// Depth of the whale's middle when its back breaks the surface, in metres.
    surface_depth: f64,
    tail_period: f64,
    tail_phase: f64,
    roll_period: f64,
    roll_phase: f64,
}

impl WhalePath {
    fn new(cove: CoveSeed, creature_id: &str, look: WhaleLook) -> Self {
        let species = look.species;
        let own = combine(cove.0, stable_hash(creature_id));
        let mut random = SplitMix64::new(combine(own, 0x9E37));
        let (route, behind, beside, beside_angle, above, breath_delay);
        if species.travels_in_pods() {
            // One route per cove and species, so every orca in the cove shares it. The hash input uses the
            // species' Swift raw value (right whale 0, orca 1).
            route = Route::new(
                combine(cove.0, stable_hash(&format!("pod-{}", species as i64))),
                species,
            );
            behind = random.range(0.0, 30.0);
            beside = random.range(3.0, 12.0);
            beside_angle = random.angle();
            above = random.range(-2.0, 2.0);
            breath_delay = random.range(0.0, 4.0);
        } else {
            route = Route::new(own, species);
            behind = 0.0;
            beside = 0.0;
            beside_angle = 0.0;
            above = 0.0;
            breath_delay = 0.0;
        }
        let wander_period = random.range(110.0, 170.0);
        let wander_phase = random.angle();
        let surface_depth = 0.085 * look.length();
        let tail_period = random.range(4.5, 6.5);
        let tail_phase = random.angle();
        let roll_period = random.range(70.0, 130.0);
        let roll_phase = random.angle();
        Self {
            species,
            route,
            behind,
            beside,
            beside_angle,
            above,
            breath_delay,
            wander_period,
            wander_phase,
            surface_depth,
            tail_period,
            tail_phase,
            roll_period,
            roll_phase,
        }
    }

    /// Orcas travel more slowly at night than by day: their clock runs 13 % fast at noon and 13 % slow at
    /// midnight, without ever jumping. Right whales keep the same pace.
    fn travel_time(&self, t: f64) -> f64 {
        if self.species != WhaleSpecies::Orca {
            return t;
        }
        let day = 86_400.0;
        t - 0.13 * day / (2.0 * PI) * cycle(t, day).sin()
    }

    fn breathing(&self, t: f64) -> Breathing {
        let route = &self.route;
        let cycles = (t - self.breath_delay + route.breathing_offset) / route.breathing_cycle;
        let phase = cycles - cycles.floor();
        let signed = if phase > 0.5 { phase - 1.0 } else { phase }; // the surface series is centred on 0
        let half = route.breaths as f64 * route.breath_interval / 2.0 / route.breathing_cycle;
        let ramp = (45.0f64).min(route.breathing_cycle * 0.2) / route.breathing_cycle; // going up or down
        let at_surface = 1.0 - sky::smooth(half, half + ramp, signed.abs());

        // Between breaths the whale dips a little; at each breath its back breaks the surface.
        let u = ((signed + half) / (2.0 * half)).max(0.0).min(1.0);
        let dip = 0.5 + 0.5 * (2.0 * PI * route.breaths as f64 * u).cos();
        let surface_y = -self.surface_depth * (1.0 + 2.2 * dip);

        // Each dive goes to its own depth.
        let dive = cycles.floor().max(0.0) as u64;
        let mut random = SplitMix64::new(combine(route.seed, dive));
        let depth = random.unit();
        let mut dive_centre = -22.0 + 12.0 * depth;
        if self.species == WhaleSpecies::Orca {
            // Orcas dive deep far less often at night.
            let daylight = sky::smooth(-6.0, 10.0, sky::sun_elevation(sky::hour(t)));
            dive_centre = (-14.0 + 8.0 * depth) * (1.0 - daylight) + dive_centre * daylight;
        }
        let dive_y =
            dive_centre + self.above + 3.5 * (cycle(t, route.period_y) + route.phase_y).sin();
        Breathing {
            at_surface,
            surface_y,
            dive_y,
        }
    }

    fn position(&self, t: f64) -> [f64; 3] {
        let along = self.travel_time(t) - self.behind;
        let mut ground = self.route.ground(along);
        ground[0] += self.beside_angle.cos() * self.beside;
        ground[1] += self.beside_angle.sin() * self.beside;
        let wander = cycle(t, self.wander_period) + self.wander_phase;
        // Gentler than the slowest part of the route, so it can never bring the whale to a stop.
        ground[0] += 0.5 * wander.sin();
        ground[1] += 0.5 * (wander + 1.3).sin();
        let b = self.breathing(t);
        [
            ground[0],
            b.dive_y + (b.surface_y - b.dive_y) * b.at_surface,
            ground[1],
        ]
    }

    /// Velocity in metres per second, measured over two seconds so it is steady.
    fn velocity(&self, t: f64) -> [f64; 3] {
        let (a, b) = (self.position(t + 1.0), self.position(t - 1.0));
        [
            (a[0] - b[0]) / 2.0,
            (a[1] - b[1]) / 2.0,
            (a[2] - b[2]) / 2.0,
        ]
    }

    /// The compass direction the whale points in, in radians: the direction it swims in, averaged over about
    /// 16 seconds, so it turns slowly where its path doubles back.
    fn heading(&self, t: f64) -> f64 {
        let (mut sx, mut sz) = (0.0, 0.0);
        let mut offset = -8.0;
        while offset <= 8.0 {
            let v = self.velocity(t + offset);
            let ground = (v[0] * v[0] + v[2] * v[2]).sqrt();
            if ground > 1e-9 {
                sx += v[0] / ground;
                sz += v[2] / ground;
            }
            offset += 2.0;
        }
        sx.atan2(sz)
    }
}
