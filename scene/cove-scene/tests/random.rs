use cove_scene::{stable_hash, SplitMix64};

#[test]
fn splitmix_matches_reference() {
    // Vigna's reference vector
    let mut g = SplitMix64::new(1234567);
    assert_eq!(g.next(), 6457827717110365317);
    assert_eq!(g.next(), 3203168211198807973);
}

#[test]
fn fnv1a_matches_reference() {
    assert_eq!(stable_hash(""), 0xcbf29ce484222325);
    assert_eq!(stable_hash("a"), 0xaf63dc4c8601ec8c);
}

#[test]
fn unit_in_range() {
    let mut g = SplitMix64::new(9);
    for _ in 0..10_000 {
        let u = g.unit();
        assert!((0.0..1.0).contains(&u));
    }
}

#[test]
fn int_inclusive() {
    let mut g = SplitMix64::new(3);
    let v: Vec<i64> = (0..1000).map(|_| g.int(2, 4)).collect();
    assert!(v.contains(&2) && v.contains(&4) && v.iter().all(|x| (2..=4).contains(x)));
}
