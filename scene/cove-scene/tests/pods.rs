mod common;
use common::*;

#[test]
fn orcas_visiting_together_stay_together() {
    let p = pod();
    let a = track("orca-1", &p, NOON, DAY, 120.0);
    let b = track("orca-2", &p, NOON, DAY, 120.0);
    let apart = a
        .iter()
        .zip(&b)
        .map(|(x, y)| dist(x, y))
        .fold(0.0f64, f64::max);
    println!("furthest apart {apart:.1} m (<45)");
    assert!(apart < 45.0);
}

#[test]
fn right_whale_goes_its_own_way() {
    let p = pod();
    let a = track("orca-1", &p, NOON, DAY, 120.0);
    let r = track("right-1", &p, NOON, DAY, 120.0);
    let far = a
        .iter()
        .zip(&r)
        .map(|(x, y)| dist(x, y))
        .fold(0.0f64, f64::max);
    println!("up to {far:.1} m from the pod (>80)");
    assert!(far > 80.0);
}

fn breath_times(id: &str) -> Vec<f64> {
    let scene = munich_bay();
    let p = pod();
    let (mut times, mut was_up) = (vec![], false);
    for t in stride(HOUR, 1.0) {
        let Some(w) = scene
            .frame(NOON + t, &p, &[])
            .whales
            .into_iter()
            .find(|w| w.id == id)
        else {
            continue;
        };
        let up = w.position[1] > -0.1 * w.length;
        if up && !was_up {
            times.push(t);
        }
        was_up = up;
    }
    times
}

#[test]
fn pod_breathes_together() {
    let (a, b) = (breath_times("orca-1"), breath_times("orca-2"));
    let gap = a
        .iter()
        .zip(&b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f64, f64::max);
    println!("gap {gap} s (<=6), {} breaths in an hour (>40)", a.len());
    assert!(a.len() == b.len() && a.len() > 40 && gap <= 6.0);
}

#[test]
fn orca_arriving_or_leaving_does_not_move_the_others() {
    let scene = munich_bay();
    let p = pod();
    let find = |v: &[_]| {
        scene
            .frame(NOON, v, &[])
            .whales
            .into_iter()
            .find(|w| w.id == "orca-1")
    };
    assert_eq!(find(&p), find(&p[..1]));
}
