use cove_scene::camera;
use cove_scene::model::CoveSeed;
use cove_scene::schools::{self, visits, FishKind, SchoolVisit, LEAD_SPEED};

const DAY_START: f64 = 1_794_000_000.0 - (1_794_000_000.0 % 86_400.0);

/// Every visit that starts within a day (found by walking the day in steps no longer than a visit).
fn day_of_visits(seed: CoveSeed) -> Vec<SchoolVisit> {
    let mut all: Vec<SchoolVisit> = Vec::new();
    let mut t = DAY_START;
    while t < DAY_START + 86_400.0 {
        for v in visits(seed, t) {
            if v.start >= DAY_START
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

#[test]
fn big_school_visits_every_few_minutes() {
    let all = day_of_visits(CoveSeed(5));
    let big = of_kind(&all, FishKind::Baitfish);
    assert!(big.len() > 100);
    for w in big.windows(2) {
        let gap = w[1].start - w[0].start;
        assert!((315.0..=585.0).contains(&gap), "gap {gap}");
    }
    for v in &big {
        let d = v.end - v.start;
        assert!((60.0..=300.0).contains(&d), "duration {d}");
        assert!((300..=400).contains(&v.count));
    }
    let circling = big.iter().filter(|v| v.circling.is_some()).count() as f64 / big.len() as f64;
    assert!((0.3..=0.7).contains(&circling), "{circling} circle");
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
    let (s, b) = (
        share_in_water(&small),
        share_in_water(&of_kind(&all, FishKind::Baitfish)),
    );
    println!("small {s:.2}, big {b:.2} of the day");
    assert!(s > b);
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
