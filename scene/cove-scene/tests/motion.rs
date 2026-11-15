mod common;
use common::*;
use cove_scene::model::WhaleSpecies;
use cove_scene::sky;

#[test]
fn orca_never_jumps() {
    never_jumps("orca-1");
}

#[test]
fn right_whale_never_jumps() {
    never_jumps("right-1");
}

fn never_jumps(id: &str) {
    let poses = track(id, &pod(), NOON, 6.0 * HOUR, 1.0);
    let (mut step, mut turn, mut roll, mut steepest) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for w in poses.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        step = step.max(dist(a, b));
        turn = turn.max(dot(a.forward, b.forward).max(-1.0).min(1.0).acos().to_degrees());
        roll = roll.max((a.roll - b.roll).abs().to_degrees());
        steepest = steepest.max(b.forward[1].max(-1.0).min(1.0).asin().abs().to_degrees());
    }
    println!("{id}: step {step:.2} m (<2), turn {turn:.2} deg (<20), roll {roll:.2} deg (<6), tilt {steepest:.1} deg (<=30), {} poses", poses.len());
    assert!(poses.len() == 6 * 3600 && step < 2.0 && turn < 20.0 && roll < 6.0 && steepest <= 30.0);
}

fn day_start() -> f64 {
    NOON - sky::hour(NOON) * HOUR + DAY
}

#[test]
fn right_whale_is_slower_than_orca() {
    let (p, d) = (pod(), day_start());
    let right = speed_of(&track("right-1", &p, d, DAY, 60.0), 60.0);
    let orca = speed_of(&track("orca-1", &p, d, DAY, 60.0), 60.0);
    println!("right {right:.3} vs orca {orca:.3} m/s (right < 0.75 orca, > 0.15)");
    assert!(right < orca * 0.75 && right > 0.15);
}

#[test]
fn orcas_travel_slower_at_night() {
    let (p, d) = (pod(), day_start());
    let by_day = speed_of(&track("orca-1", &p, d + 11.0 * HOUR, 2.0 * HOUR, 10.0), 10.0);
    let by_night = speed_of(&track("orca-1", &p, d + 23.0 * HOUR, 2.0 * HOUR, 10.0), 10.0);
    println!("night {by_night:.3} vs day {by_day:.3} m/s (night < 0.9 day)");
    assert!(by_night < by_day * 0.9);
}

fn deepest(start: f64) -> f64 {
    track("orca-1", &pod(), start, 2.0 * HOUR, 5.0)
        .iter()
        .map(|w| w.position[1])
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn orcas_dive_less_deep_at_night() {
    let d = day_start();
    let (night, day) = (deepest(d + 23.0 * HOUR), deepest(d + 11.0 * HOUR));
    println!("night {:.1} m vs day {:.1} m (night > day + 3)", -night, -day);
    assert!(night > day + 3.0);
}

#[test]
fn right_whale_echo_calls_more_at_night() {
    let scene = munich_bay();
    let e = [echo("e", day_start() - HOUR)];
    let calling = |start: f64| {
        stride(4.0 * HOUR, 1.0)
            .filter(|t| scene.frame(start + t, &[], &e).echoes[0].call.is_some())
            .count()
    };
    let (night, day) = (calling(day_start() + 22.0 * HOUR), calling(day_start() + 10.0 * HOUR));
    println!("night {night} s vs day {day} s in four hours");
    assert!(night > day);
}

#[test]
fn whales_move_smoothly_from_one_frame_to_the_next() {
    let scene = munich_bay();
    let now = scene.frame(NOON, &guests(), &[]).whales;
    let next = scene.frame(NOON + 1.0 / 15.0, &guests(), &[]).whales;
    let (mut d_max, mut r_max) = (0.0f64, 0.0f64);
    for (a, b) in now.iter().zip(&next) {
        d_max = d_max.max(dist(a, b));
        r_max = r_max.max((a.roll - b.roll).abs());
    }
    println!("max move {d_max:.4} m (<0.2), max roll change {r_max:.4} (<0.05)");
    assert!(now.iter().zip(&next).all(|(a, b)| a.id == b.id && dist(a, b) < 0.2 && (a.roll - b.roll).abs() < 0.05));
}

#[test]
fn visitor_is_the_whale_its_creator_chose() {
    let whales = munich_bay().frame(NOON, &guests(), &[]).whales;
    let b = whales.iter().find(|w| w.id == "whale-b").unwrap();
    assert!(b.species == WhaleSpecies::Orca && b.length == 7.0);
}
