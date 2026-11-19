use cove_scene::camera;
use cove_scene::model::CoveSeed;
use cove_scene::schools::{self, visits, FishKind, SchoolVisit, LEAD_SPEED};

const DAY_START: f64 = 1_794_000_000.0 - (1_794_000_000.0 % 86_400.0);

/// Every scheduled visit (not the resident school) that starts within a day (found by walking the day in steps no longer than a visit).
fn day_of_visits(seed: CoveSeed) -> Vec<SchoolVisit> {
    let mut all: Vec<SchoolVisit> = Vec::new();
    let mut t = DAY_START;
    while t < DAY_START + 86_400.0 {
        for v in visits(seed, t) {
            if v.wander.is_none()
                && v.start >= DAY_START
                && v.start < DAY_START + 86_400.0
                && !all.iter().any(|o| o.id == v.id)
            {
                all.push(v);
            }
        }
        t += 20.0;
    }
    all.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    all
}

fn of_kind(all: &[SchoolVisit], kind: FishKind) -> Vec<&SchoolVisit> {
    all.iter().filter(|v| v.kind == kind).collect()
}

fn share_in_water(all: &[&SchoolVisit]) -> f64 {
    all.iter().map(|v| v.end - v.start).sum::<f64>() / 86_400.0
}

#[test]
fn visits_are_shared() {
    for i in 0..50 {
        let t = DAY_START + i as f64 * 1_731.0;
        assert_eq!(visits(CoveSeed(5), t), visits(CoveSeed(5), t));
    }
    assert!((0..200).any(|i| {
        let t = DAY_START + i as f64 * 431.0;
        visits(CoveSeed(5), t) != visits(CoveSeed(6), t)
    }));
}

#[test]
fn visits_under_way_are_listed_by_start() {
    for i in 0..400 {
        let t = DAY_START + i as f64 * 211.0;
        let now = visits(CoveSeed(9), t);
        assert!(now.iter().all(|v| v.start <= t && t < v.end));
        assert!(now.windows(2).all(|w| w[0].start <= w[1].start));
    }
}

// The resident big school (owner, 2026-11-19).
fn resident(seed: u64, at: f64) -> SchoolVisit {
    let big: Vec<SchoolVisit> = visits(CoveSeed(seed), at)
        .into_iter()
        .filter(|v| v.kind == FishKind::Baitfish)
        .collect();
    assert_eq!(big.len(), 1, "one big school at {at}");
    big[0].clone()
}

#[test]
fn big_school_is_always_there() {
    let first = resident(5, DAY_START);
    assert!((300..=400).contains(&first.count) && first.wander.is_some());
    for i in 0..200 {
        let v = resident(5, DAY_START + i as f64 * 977.0);
        assert_eq!(v.id, first.id, "the same school every day");
    }
}

#[test]
fn big_school_is_mostly_in_the_middle_distance() {
    let v = resident(5, DAY_START);
    let (mut far, mut near, mut n) = (0.0, 0.0, 0.0);
    let mut t = DAY_START;
    while t < DAY_START + 10.0 * 86_400.0 {
        let ahead = camera::ahead(v.lead(t));
        if ahead >= 38.0 {
            far += 1.0;
        } else if ahead <= 12.0 {
            near += 1.0;
        }
        n += 1.0;
        t += 10.0;
    }
    let (far, near) = (far / n, near / n);
    println!("far {far:.2}, near {near:.2}");
    assert!((0.1..=0.3).contains(&far) && (0.1..=0.3).contains(&near));
}

#[test]
fn big_school_visits_the_whole_picture_every_hour() {
    let v = resident(5, DAY_START);
    for hour in 0..6 {
        let mut cells = [false; 9];
        for s in 0..720 {
            let t = DAY_START + hour as f64 * 3_600.0 + s as f64 * 5.0;
            if let Some((x, y, _)) = camera::project(v.lead(t), 1.6) {
                if (0.0..1.0).contains(&x) && (0.0..1.0).contains(&y) {
                    cells[(x * 3.0) as usize + 3 * (y * 3.0) as usize] = true;
                }
            }
        }
        // The top row far away lies above the surface, where the lead may not go; the other cells all count.
        assert!(cells[..6].iter().all(|&c| c), "hour {hour}: {cells:?}");
    }
}

#[test]
fn big_school_is_always_moving_slowly() {
    let v = resident(5, DAY_START);
    for s in 0..20_000 {
        let t = DAY_START + s as f64;
        let (a, b) = (v.lead(t), v.lead(t + 1.0));
        let step = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        assert!(step <= 1.0, "{step} m/s at {t}");
        assert!(a[1] <= -3.0);
    }
}

#[test]
fn small_school_visits_more_often() {
    let all = day_of_visits(CoveSeed(5));
    let small = of_kind(&all, FishKind::Mackerel);
    for w in small.windows(2) {
        let gap = w[1].start - w[0].start;
        assert!((240.0..=360.0).contains(&gap), "gap {gap}");
        assert!(w[1].start - w[0].end >= 30.0, "visits end too late");
    }
    for v in &small {
        assert!((30..=60).contains(&v.count));
        assert!(v.circling.is_none());
    }
    let s = share_in_water(&small);
    println!("small school in the water {s:.2} of the day");
    assert!((0.3..=0.8).contains(&s));
}

#[test]
fn routes_enter_and_leave_out_of_sight() {
    for seed in [5, 6, 77] {
        for v in day_of_visits(CoveSeed(seed)) {
            for at in [v.start, v.end] {
                for aspect in [0.5625, 2.333] {
                    let (x, _, _) = camera::project(v.lead(at), aspect).expect("in front");
                    assert!(!(0.0..=1.0).contains(&x), "{:?} at {at}: x {x}", v.kind);
                }
            }
        }
    }
}

#[test]
fn lead_moves_smoothly() {
    for v in day_of_visits(CoveSeed(5)) {
        let mut t = v.start;
        while t < v.end {
            let (a, b) = (v.lead(t), v.lead(t + 1.0));
            let step =
                ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
            assert!(step <= 1.5 * LEAD_SPEED, "step {step} m/s");
            assert!(a[1] <= -2.0, "{} m deep", -a[1]);
            let camera = (a[0].powi(2) + (a[1] + 30.0).powi(2) + a[2].powi(2)).sqrt();
            assert!(camera >= 3.0, "{camera} m from the camera");
            t += 1.0;
        }
    }
}

// The preview's `--school big|small`: a visit of the chosen kind under way at the chosen moment, five
// seconds in, with the schedule's route rules.
#[test]
fn forced_visit_is_under_way() {
    for kind in [FishKind::Baitfish, FishKind::Mackerel] {
        let at = 1_794_000_000.0;
        let v = schools::forced_visit(CoveSeed(5), kind, at);
        assert_eq!(v.kind, kind);
        assert!((v.start - (at - 5.0)).abs() < 1e-9 && v.end > at + 60.0);
        let x = camera::project(v.lead(v.start), 2.333).map_or(-1.0, |p| p.0);
        assert!(!(0.0..=1.0).contains(&x), "enters out of sight");
    }
}
