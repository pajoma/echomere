use cove_scene::camera;
use cove_scene::model::scene_rules;
use cove_scene::scene::FishPose;
use cove_scene::schools::{Circling, FishKind, SchoolVisit, LEAD_SPEED};
use cove_swarm::{milling, polarisation, Swarm};

const T0: f64 = 1_794_000_000.0;
const FRAME: f64 = 1.0 / 15.0;

/// A point `ahead` metres along the view axis at `depth` metres, `sideways` x `ahead` to the right (as the
/// schedule places its routes).
fn water(ahead: f64, depth: f64, sideways: f64) -> [f64; 3] {
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

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn overshoot(kind: FishKind) -> f64 {
    match kind {
        FishKind::Baitfish => 20.0,
        FishKind::Mackerel => 10.0,
    }
}

/// A visit straight across the picture, from the left (`side` -1) or the right (`side` 1).
fn crossing(id: u64, kind: FishKind, count: usize, start: f64, side: f64) -> SchoolVisit {
    let entry = water(20.0, 14.0, 2.2 * side);
    let exit = water(20.0, 14.0, -2.2 * side);
    SchoolVisit {
        id,
        kind,
        count,
        start,
        end: start + (distance(entry, exit) + overshoot(kind)) / LEAD_SPEED,
        entry,
        exit,
        circling: None,
    }
}

/// A baitfish visit that circles overhead for two minutes on its way across, on the widest circle the
/// schedule draws (10 m).
fn circling(id: u64, start: f64) -> SchoolVisit {
    let mut visit = crossing(id, FishKind::Baitfish, 350, start, -1.0);
    let (centre, radius) = (water(22.0, 14.0, 0.0), 10.0);
    let joined = start + (distance(visit.entry, centre) - radius) / LEAD_SPEED;
    let circle = Circling {
        start: joined,
        end: joined + 120.0,
        centre,
        radius,
    };
    visit.end = circle.end + (distance(circle.centre, visit.exit) + 30.0) / LEAD_SPEED;
    visit.circling = Some(circle);
    visit
}

/// The visits under way at a moment, as `schools::visits` gives them.
fn under_way(all: &[SchoolVisit], at: f64) -> Vec<SchoolVisit> {
    all.iter()
        .filter(|v| v.start <= at && at < v.end)
        .cloned()
        .collect()
}

/// Advances the swarm frame by frame from `from` to `to`, calling `each` after every frame.
fn run(
    swarm: &mut Swarm,
    all: &[SchoolVisit],
    from: f64,
    to: f64,
    mut each: impl FnMut(f64, &Swarm),
) {
    let frames = ((to - from) / FRAME).round() as usize;
    for frame in 0..=frames {
        let at = from + frame as f64 * FRAME;
        swarm.advance(&under_way(all, at), at);
        each(at, swarm);
    }
}

fn of_kind(fish: &[FishPose], kind: FishKind) -> Vec<FishPose> {
    fish.iter().filter(|f| f.kind == kind).cloned().collect()
}

fn centre(fish: &[FishPose]) -> [f64; 3] {
    let mut sum = [0.0; 3];
    for f in fish {
        for k in 0..3 {
            sum[k] += f.position[k];
        }
    }
    sum.map(|s| s / fish.len() as f64)
}

/// The closest two fish come, as a share of the larger one's body length.
fn closest(fish: &[FishPose]) -> f64 {
    let mut least = f64::INFINITY;
    for (i, a) in fish.iter().enumerate() {
        for b in &fish[i + 1..] {
            least = least.min(distance(a.position, b.position) / a.size.max(b.size));
        }
    }
    least
}

/// Whether the swarm is due at a whole second since `from` (to sample once a second).
fn on_the_second(at: f64, from: f64) -> bool {
    let t = (at - from) / FRAME;
    (t.round() as i64) % 15 == 0
}

#[test]
fn school_forms_parallel_group() {
    let visit = crossing(1, FishKind::Baitfish, 350, T0, -1.0);
    let mut swarm = Swarm::new(7);
    run(&mut swarm, &[visit], T0, T0 + 20.0, |_, _| {});
    let fish = swarm.poses();
    assert_eq!(fish.len(), 350);
    let p = polarisation(&fish);
    println!("polarisation {p:.3}");
    assert!(p > 0.8, "polarisation {p}");
}

#[test]
fn school_mills_in_torus() {
    let visit = circling(1, T0);
    let from = visit.circling.expect("circles").start + 5.0;
    let mut swarm = Swarm::new(7);
    run(&mut swarm, &[visit], from, from + 40.0, |_, _| {});
    let fish = swarm.poses();
    let (m, p) = (milling(&fish), polarisation(&fish));
    println!("milling {m:.3}, polarisation {p:.3}");
    assert!(m > 0.6, "milling {m}");
    assert!(p < 0.5, "polarisation {p}");
}

#[test]
fn school_stays_together() {
    let visit = crossing(1, FishKind::Baitfish, 350, T0, 1.0);
    let mut swarm = Swarm::new(11);
    let mut widest: f64 = 0.0;
    run(&mut swarm, &[visit], T0, T0 + 60.0, |at, swarm| {
        if on_the_second(at, T0) {
            let fish = swarm.poses();
            let c = centre(&fish);
            for f in &fish {
                widest = widest.max(distance(f.position, c));
            }
        }
    });
    println!("farthest fish {widest:.2} m from the centre");
    assert!(widest <= 25.0, "{widest} m");
}

#[test]
fn fish_keep_their_distance() {
    let visit = crossing(1, FishKind::Baitfish, 350, T0, -1.0);
    let mut swarm = Swarm::new(3);
    let mut least = f64::INFINITY;
    run(&mut swarm, &[visit], T0, T0 + 60.0, |at, swarm| {
        if on_the_second(at, T0) {
            least = least.min(closest(&swarm.poses()));
        }
    });
    println!("closest {least:.3} body lengths");
    assert!(least >= 0.5, "{least} body lengths");
}

#[test]
fn schools_do_not_overlap() {
    // Both schools on the same route at the same time, so they share the water.
    let big = crossing(1, FishKind::Baitfish, 350, T0, -1.0);
    let small = crossing(2, FishKind::Mackerel, 50, T0, -1.0);
    let mut swarm = Swarm::new(5);
    let mut least = f64::INFINITY;
    run(&mut swarm, &[big, small], T0, T0 + 60.0, |at, swarm| {
        if on_the_second(at, T0) {
            let fish = swarm.poses();
            let (b, s) = (
                of_kind(&fish, FishKind::Baitfish),
                of_kind(&fish, FishKind::Mackerel),
            );
            assert_eq!((b.len(), s.len()), (350, 50));
            for x in &b {
                for y in &s {
                    least = least.min(distance(x.position, y.position) / x.size.max(y.size));
                }
            }
        }
    });
    println!("closest across schools {least:.3} body lengths");
    assert!(least >= 0.5, "{least} body lengths");
}

#[test]
fn informed_fish_lead_the_school() {
    for visit in [
        circling(1, T0),
        crossing(2, FishKind::Mackerel, 50, T0, 1.0),
    ] {
        let mut swarm = Swarm::new(9);
        let mut farthest: f64 = 0.0;
        let all = [visit.clone()];
        run(
            &mut swarm,
            &all,
            visit.start,
            visit.end - FRAME,
            |at, swarm| {
                if on_the_second(at, visit.start) {
                    let c = centre(&swarm.poses());
                    farthest = farthest.max(distance(c, visit.lead(at)));
                }
            },
        );
        println!(
            "{:?}: centre at most {farthest:.2} m from the lead",
            visit.kind
        );
        assert!(farthest <= 12.0, "{farthest} m");
    }
}

fn formed_near_the_lead(swarm: &Swarm, visit: &SchoolVisit, at: f64) {
    let fish = swarm.poses();
    assert_eq!(fish.len(), visit.count);
    let (p, d) = (polarisation(&fish), distance(centre(&fish), visit.lead(at)));
    println!("polarisation {p:.3}, {d:.2} m from the lead");
    assert!(p > 0.8, "polarisation {p}");
    assert!(d <= 12.0, "{d} m from the lead");
}

#[test]
fn school_starts_mid_visit() {
    for visit in [
        crossing(1, FishKind::Baitfish, 350, T0, -1.0),
        crossing(2, FishKind::Mackerel, 50, T0, 1.0),
    ] {
        let at = visit.start + 0.5 * (visit.end - visit.start);
        let mut swarm = Swarm::new(13);
        swarm.advance(std::slice::from_ref(&visit), at);
        formed_near_the_lead(&swarm, &visit, at);
    }
}

#[test]
fn swarm_restarts_after_a_gap() {
    let first = crossing(1, FishKind::Baitfish, 350, T0, -1.0);
    let later = crossing(2, FishKind::Baitfish, 320, T0 + 7200.0, 1.0);
    let mut swarm = Swarm::new(17);
    let at = T0 + 40.0;
    swarm.advance(&[first], at);
    let at = at + 7200.0;
    swarm.advance(std::slice::from_ref(&later), at);
    formed_near_the_lead(&swarm, &later, at);
}

#[test]
fn school_is_gone_only_out_of_sight() {
    for visit in [
        crossing(1, FishKind::Baitfish, 350, T0, -1.0),
        crossing(2, FishKind::Mackerel, 50, T0, 1.0),
    ] {
        let all = [visit.clone()];
        let mut swarm = Swarm::new(19);
        let last = visit.start + ((visit.end - visit.start) / FRAME).ceil() * FRAME - FRAME;
        run(&mut swarm, &all, visit.start, last, |_, _| {});
        let fish = swarm.poses();
        assert_eq!(fish.len(), visit.count);
        for f in &fish {
            if let Some((x, y, _)) = camera::project(f.position, 1.6) {
                assert!(
                    !((0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y)),
                    "{:?} fish in the picture at ({x:.2}, {y:.2}) as the visit ends",
                    visit.kind
                );
            }
        }
        swarm.advance(&under_way(&all, last + FRAME), last + FRAME);
        assert!(swarm.poses().is_empty());
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore)]
fn simulation_fits_the_budget() {
    let big = crossing(1, FishKind::Baitfish, 400, T0, -1.0);
    let mut small = crossing(2, FishKind::Mackerel, 60, T0, 1.0);
    small.entry = water(28.0, 20.0, 2.2);
    small.exit = water(28.0, 20.0, -2.2);
    let all = [big, small];
    let mut swarm = Swarm::new(23);
    let from = T0 + 30.0;
    swarm.advance(&all, from);
    let started = std::time::Instant::now();
    for frame in 1..=150 {
        swarm.advance(&all, from + frame as f64 * FRAME);
    }
    let each = started.elapsed().as_secs_f64() * 1000.0 / 150.0;
    assert_eq!(swarm.poses().len(), 460);
    println!("{each:.3} ms a frame");
    assert!(each < 2.0, "{each} ms a frame");
}
