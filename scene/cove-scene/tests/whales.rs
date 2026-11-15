mod common;
use common::*;
use cove_scene::camera;
use cove_scene::model::{scene_rules, WhaleLook, WhaleSpecies};

#[test]
fn whale_not_in_scene_before_it_arrives() {
    let late = visitor("late", NOON + 60.0, WhaleSpecies::SouthernRightWhale);
    assert!(munich_bay().frame(NOON, &[late], &[]).whales.is_empty());
}

#[test]
fn whale_fades_in_over_two_minutes() {
    let arriving = [visitor("new", NOON, WhaleSpecies::SouthernRightWhale)];
    let o: Vec<f64> = [0.0, 60.0, 120.0, 600.0]
        .iter()
        .map(|d| munich_bay().frame(NOON + d, &arriving, &[]).whales.first().map_or(-1.0, |w| w.opacity))
        .collect();
    println!("opacities {o:?}");
    assert!(o[0] == 0.0 && o[1] > 0.4 && o[1] < 0.6 && o[2] == 1.0 && o[3] == 1.0);
}

#[test]
fn at_most_six_whales_are_drawn() {
    let crowd: Vec<_> = (0..8)
        .map(|i| visitor(&format!("crowd-{i}"), NOON - (8 - i) as f64 * HOUR, WhaleSpecies::SouthernRightWhale))
        .collect();
    let crowded = munich_bay().frame(NOON, &crowd, &[]);
    assert_eq!(crowded.whales.len(), scene_rules::MAX_WHALES);
}

struct DayStats {
    in_view_share: f64,
    up_front_share: f64,
    up_front: usize,
    fastest: f64,
    shallowest: f64,
    deepest: f64,
}

fn day_stats() -> DayStats {
    let scene = munich_bay();
    let g = guests();
    let (mut in_view, mut samples, mut up_front) = (0usize, 0usize, 0usize);
    let (mut fastest, mut shallowest, mut deepest) = (0.0f64, -100.0f64, 0.0f64);
    let mut previous: Option<cove_scene::scene::WhalePose> = None;
    for step in stride(DAY, 20.0) {
        let Some(whale) = scene.frame(NOON + step, &g[..1], &[]).whales.into_iter().next() else { continue };
        samples += 1;
        if let Some((x, y, d)) = camera::project(whale.position, 1.6) {
            if x > -0.1 && x < 1.1 && y > -0.1 && y < 1.1 && d < 110.0 {
                in_view += 1;
            }
        }
        if whale.position[2] < 32.0 {
            up_front += 1;
        }
        shallowest = shallowest.max(whale.position[1]);
        deepest = deepest.min(whale.position[1]);
        if let Some(p) = &previous {
            fastest = fastest.max(dist(&whale, p) / 20.0);
        }
        previous = Some(whale);
    }
    DayStats {
        in_view_share: in_view as f64 / samples as f64,
        up_front_share: up_front as f64 / samples as f64,
        up_front,
        fastest,
        shallowest,
        deepest,
    }
}

#[test]
fn whale_roams_in_and_out_of_view() {
    let s = day_stats();
    println!("in view {:.1}% (0.3..0.85)", s.in_view_share * 100.0);
    assert!(s.in_view_share > 0.3 && s.in_view_share < 0.85);
}

#[test]
fn whale_comes_to_the_front_only_rarely() {
    let s = day_stats();
    println!("up front {:.2}% (<5%)", s.up_front_share * 100.0);
    assert!(s.up_front > 0 && s.up_front_share < 0.05);
}

#[test]
fn whale_never_leaves_water_or_touches_seabed() {
    let s = day_stats();
    println!("{:.2} to {:.1} m deep", -s.shallowest, -s.deepest);
    assert!(s.shallowest < -0.5 && s.deepest > -scene_rules::CAMERA_DEPTH + 2.0);
}

#[test]
fn whale_swims_at_a_whales_pace() {
    let s = day_stats();
    println!("fastest {:.2} m/s (<2)", s.fastest);
    assert!(s.fastest < 2.0);
}

fn breathing_stats() -> (usize, usize, f64) {
    let scene = munich_bay();
    let g = guests();
    let (mut breaths, mut visits, mut was_up, mut last, mut deepest) = (0usize, 0usize, false, -1000.0f64, 0.0f64);
    for step in stride(2.0 * HOUR, 1.0) {
        let Some(w) = scene.frame(NOON + step, &g[..1], &[]).whales.into_iter().next() else { continue };
        let up = w.position[1] > -0.1 * w.length;
        if up && !was_up {
            breaths += 1;
            if step - last > 120.0 {
                visits += 1;
            }
            last = step;
        }
        was_up = up;
        deepest = deepest.min(w.position[1]);
    }
    (breaths, visits, deepest)
}

#[test]
fn whale_breathes_several_times_at_a_time() {
    let (b, v, _) = breathing_stats();
    println!("{b} breaths in {v} visits (visits 8..=20, breaths 4..=6 per visit)");
    assert!(v >= 8 && v <= 20 && b >= v * 4 && b <= v * 6);
}

#[test]
fn whale_dives_deep_between_breaths() {
    let (_, _, d) = breathing_stats();
    println!("deepest {:.1} m (<-15)", d);
    assert!(d < -15.0);
}

#[test]
fn orca_breathes_more_often_than_right_whale() {
    let orca = WhaleLook::new(WhaleSpecies::Orca, 1.0);
    let right = WhaleLook::new(WhaleSpecies::SouthernRightWhale, 1.0);
    assert!(orca.species.breathing_cycle().1 < right.species.breathing_cycle().0);
}
