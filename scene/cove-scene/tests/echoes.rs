mod common;
use common::*;
use cove_scene::model::scene_rules;

#[test]
fn echo_older_than_seven_days_is_gone() {
    let f = munich_bay().frame(NOON, &[], &[echo("old", NOON - 7.0 * DAY - 1.0)]);
    assert!(f.echoes.is_empty());
}

#[test]
fn cove_shows_at_most_its_five_most_recent_echoes() {
    let many: Vec<_> = (0..8).map(|i| echo(&format!("e{i}"), NOON - (i + 1) as f64 * HOUR)).collect();
    let scene = munich_bay();
    let with = scene.frame(NOON, &[], &many);
    let first5 = scene.frame(NOON, &[], &many[..5]);
    let a: Vec<f64> = with.echoes.iter().map(|e| e.intensity).collect();
    let b: Vec<f64> = first5.echoes.iter().map(|e| e.intensity).collect();
    assert!(with.echoes.len() == scene_rules::MAX_ECHOES && a == b);
}

#[test]
fn echo_grows_fainter_as_it_ages() {
    let scene = munich_bay();
    let fresh = scene.frame(NOON, &[], &[echo("e", NOON - HOUR)]).echoes[0].intensity;
    let older = scene.frame(NOON, &[], &[echo("e", NOON - 5.0 * DAY)]).echoes[0].intensity;
    println!("fresh {fresh:.4}, older {older:.4}");
    assert!(fresh > older && older > 0.0);
}

#[test]
fn echo_calls_now_and_then() {
    let scene = munich_bay();
    let e = [echo("e", NOON - HOUR)];
    let calls = stride(2.0 * HOUR, 1.0)
        .filter(|t| scene.frame(NOON + t, &[], &e).echoes[0].call.is_some())
        .count();
    println!("{calls} s of calling in two hours (0 < n < 120)");
    assert!(calls > 0 && calls < 120);
}
