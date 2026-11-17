# Kelp as a Foreground Frame Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the background giant kelp with 3–6 long ribbon blades per side that frame the picture's
left and right edges, moving with abyssal-garden's `W_LEAF` blade animation at a meditative pace.

**Architecture:** `cove-scene` generates the blades once per cove (own random stream) and gives every frame
a `KelpMotion` (swell surge angle + slowed kelp clock). `cove-render`'s existing kelp pass keeps its
instanced, depth-tested pipeline; its mesh becomes one ribbon strip, its instance data the blade, and its
vertex shader a port of `W_LEAF`'s `vs`, lit by today's water-model fragment.

**Tech Stack:** Rust (workspace `scene/`), wgpu, WGSL.

**Spec:** `docs/superpowers/specs/2026-11-16-kelp-vignette-design.md`

## Global Constraints

- Frame = f(seed, visitors, echoes, time); two screens of one cove show the same kelp (ADR 0005).
- No platform `#[cfg]` in `cove-scene`, `cove-render` or shaders (ADR 0008).
- No more kelp vertices than today (16 × 64 × 9 rows, about 28 000).
- abyssal-garden code keeps its attribution: "Adapted from abyssal-garden, Copyright (c) 2026 Maksymilian
  Dendura, MIT License" plus the `shaders.js` lines, as elsewhere in the shaders.
- Tests: `cd scene && $HOME/.cargo/bin/cargo test --workspace -- --test-threads=1`; `cargo fmt --check` and
  `cargo clippy --workspace --all-targets` clean.
- Commits on `feature/7-rust-port`; message ends with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. Screens of other shapes (4:3, 21:9, a portrait 9:16 monitor): the frame stays at both edges and the
   middle stays free — pinned by `kelp_keeps_the_middle_free` at four sizes (Task 1).
2. The swell's extremes: at the far points of the surge the blades still keep out of the middle — the same
   test samples four moments over one surge period (Task 1).
3. Blades near the camera: no blade comes closer than 2 m (near plane 0.3 m) — `kelp_frames_the_picture`
   (Task 1).
4. The daily wrap: the kelp clock repeats daily like the water's shader time, so the 400-days-later frame is
   identical — `kelp_clock_runs_slow` and the existing `shader_time_wraps_daily` (Task 1).
5. A cove whose layout changed: kelp and fish differ between coves, fish don't depend on kelp —
   `different_coves_have_different_kelp_and_fish`, `fish_ignore_the_kelp` (Task 1).

---

### Task 1: Kelp blades end to end

**Files:**
- Modify: `scene/cove-scene/src/kelp.rs` (blade type, motion, pace; remove `KelpFrond`, drift, flutter)
- Modify: `scene/cove-scene/src/layout.rs` (blades from their own stream; fish stream unchanged otherwise)
- Modify: `scene/cove-scene/src/scene.rs` (`SceneFrame.kelp: Vec<KelpBlade>`)
- Modify: `scene/cove-render/src/kelp.rs` (ribbon mesh, 48-byte instances, uniforms)
- Rewrite: `scene/cove-render/shaders/kelp.wgsl`
- Modify: `scene/scene-preview/src/main.rs` only if the kelp count print breaks
- Test: `scene/cove-scene/tests/scene.rs`, `scene/cove-scene/tests/sea.rs`, `scene/cove-render/tests/render.rs`

**Interfaces:**
- Produces in `cove_scene::kelp`:
  ```rust
  pub const KELP_PACE: f64 = 0.4;
  pub struct KelpBlade {
      pub root: [f64; 3],     // world metres (x right, y up with the surface at 0, z ahead)
      pub length: f64,        // 8..20 m
      pub lean: f64,          // signed picture-x ratio of the blade's line at 16:10 (see Step 3)
      pub forward: f64,       // -0.2..0.2: how far the blade leans away from (+) or towards the camera
      pub width: f64,         // full width at the widest point, metres
      pub curvature: f64,     // 0..1, W_LEAF's i.c.w
      pub seed: [f64; 2],     // W_LEAF's s1, s2, each 0..1
      pub side: f64,          // -1 left, +1 right
      pub sway_phase: f64,    // radians, offsets the swell push per blade
  }
  pub struct KelpMotion { pub surge: f64, pub time: f64 }
  pub fn motion(at: f64) -> KelpMotion;
  pub fn surge_period() -> f64;   // unchanged
  ```
- `CoveLayout.kelp: Vec<KelpBlade>`, `SceneFrame.kelp: Vec<KelpBlade>`.
- Instance buffer per blade, three `vec4f` (stride 48): `a` = root x, root y, root z, length;
  `b` = lean, forward, width, curvature; `c` = seed 0, seed 1, side, sway phase.
- Kelp uniforms: `u[0]` = aspect, surge, kelp time, -; `u[1]` = 1 / pixels high, warmth, -, -; `u[2]`, `u[3]`
  unchanged (light direction + intensity, light colour + ambient).

- [ ] **Step 1: Write the failing scene tests**

In `cove-scene/tests/scene.rs` replace `kelp_grows_where_giant_kelp_grows` and `kelp_is_only_hinted_at`:

```rust
#[test]
fn kelp_frames_the_picture() {
    let kelp = munich_bay().layout.kelp;
    for side in [-1.0, 1.0] {
        let n = kelp.iter().filter(|b| b.side == side).count();
        assert!((3..=6).contains(&n), "{n} blades on side {side}");
    }
    for b in &kelp {
        let (x, y, _) = camera::project(b.root, 1.6).expect("root in front of the camera");
        assert!(y < 0.0, "root below the picture, at y {y}");
        assert!(if b.side < 0.0 { x < 0.2 } else { x > 0.8 }, "root at picture x {x}");
        let ahead = camera::ahead(b.root);
        assert!((2.0..=8.0).contains(&ahead), "root {ahead} m ahead");
        assert!((8.0..=20.0).contains(&b.length));
    }
}

#[test]
fn fish_ignore_the_kelp() {
    // Kelp and fish come from separate streams of the cove seed.
    let a = CoveLayout::new(CoveSeed(7));
    assert_eq!(a.schools, CoveLayout::schools_for(CoveSeed(7)));
    assert_eq!(a.kelp, CoveLayout::blades_for(CoveSeed(7)));
}
```

(`ahead` = `(root.y + 30) * sin(PITCH) + root.z * cos(PITCH)`; expose a helper `camera::ahead(point) -> f64`
in `cove-scene/src/camera.rs` and use it in `project` too.) `CoveLayout::schools_for(seed)` and
`CoveLayout::blades_for(seed)` are public constructors of each part from its own stream.

In `cove-scene/tests/sea.rs` keep `kelp_sways_with_swell` and `kelp_sways_same_everywhere` as they are and
add:

```rust
#[test]
fn kelp_clock_runs_slow() {
    let a = kelp::motion(NOON);
    let b = kelp::motion(NOON + 1.0);
    assert!((b.time - a.time - kelp::KELP_PACE).abs() < 1e-9);
    assert!((kelp::motion(NOON + 86_400.0).time - a.time).abs() < 1e-6, "repeats daily");
    assert!((kelp::KELP_PACE - 0.4).abs() < 1e-12);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd scene && $HOME/.cargo/bin/cargo test -p cove-scene`
Expected: compile errors (`KelpBlade`, `KELP_PACE`, `schools_for`, `camera::ahead` don't exist).

- [ ] **Step 3: Implement the scene side**

- `layout.rs`: `CoveLayout::new(seed)` = `{ kelp: blades_for(seed), schools: schools_for(seed) }`.
  `schools_for` uses `SplitMix64::new(seed.0)` exactly as today minus the kelp draws; `blades_for` uses
  `SplitMix64::new(seed.0 ^ 0x6B65_6C70_6B65_6C70)` ("kelpkelp").
- Placement per blade, for side s ∈ {-1, +1}, blade count `random.int(3, 6)` per side:
  picture x of the root `px = 0.5 + s * (0.5 - random.range(0.0, 0.12))`, picture y `py = random.range(-0.6,
  -0.2)`, distance along the view axis `ahead = random.range(2.0, 8.0)`. Invert `camera::project` at aspect
  1.6: `dx = (px - 0.5) * ahead * 1.6 / FOCAL`, `up = (py - 0.5) * ahead / FOCAL`,
  `dy = up * cos(PITCH) + ahead * sin(PITCH)`, `dz = -up * sin(PITCH) + ahead * cos(PITCH)`,
  `root = [dx, dy - CAMERA_DEPTH, dz]`. `lean = (px - 0.5) * 1.6 / FOCAL` (the picture-x ratio the blade's line
  keeps). `length = random.range(8.0, 20.0)`, `forward = random.range(-0.2, 0.2)`, `width =
  random.range(0.35, 0.6)`, `curvature = random.unit()`, `seed = [random.unit(), random.unit()]`,
  `sway_phase = random.angle()`.
- `kelp.rs`: `motion(at)` = `KelpMotion { surge: cycle(at, surge_period()), time: at.rem_euclid(86_400.0) *
  KELP_PACE }`. Delete `KelpFrond`, `DRIFT_PERIOD`, `FLUTTER_PERIOD`.

- [ ] **Step 4: Run the scene tests**

Run: `cd scene && $HOME/.cargo/bin/cargo test -p cove-scene`
Expected: PASS.

- [ ] **Step 5: Write the failing render test**

Add to `cove-render/tests/render.rs` (keep `kelp_is_drawn`):

```rust
#[test]
fn kelp_keeps_the_middle_free() {
    let mut r = renderer();
    for (w, h) in [(320, 200), (320, 240), (420, 180), (180, 320)] {
        for i in 0..4 {
            let mut with = frame();
            with.kelp_motion = kelp::motion(with.time + i as f64 * kelp::surge_period() / 4.0);
            let mut without = with.clone();
            without.kelp.clear();
            let (a, b) = (r.snapshot(&with, w, h).unwrap(), r.snapshot(&without, w, h).unwrap());
            // Count changed pixels (any channel differs by more than 2) in: the left 20 % band, the
            // right 20 % band, and the middle.
            let (left, right, middle) = /* ... */;
            let band = (w as f64 * 0.2).floor() * h as f64;
            let total = (left + right + middle) as f64;
            assert!(middle as f64 <= 0.05 * total, "{w}x{h} #{i}: {middle} of {total} in the middle");
            assert!(left as f64 >= 0.02 * band && right as f64 >= 0.02 * band, "{w}x{h} #{i}: {left} / {right}");
        }
    }
}
```

- [ ] **Step 6: Run it to see it fail**

Run: `cd scene && $HOME/.cargo/bin/cargo test -p cove-render --test render kelp_ -- --test-threads=1`
Expected: compile error or FAIL (today's kelp is spread across the middle).

- [ ] **Step 7: Implement the renderer side**

- `cove-render/src/kelp.rs`: the mesh is one ribbon strip of `BLADE_ROWS = 48` rows × 2 vertices, each
  vertex `(u, row)` with u = -1 or 1; indices two triangles per row step. Instances as in Interfaces; write
  `KelpMotion.surge` and `.time` into `u[0].y`, `u[0].z`.
- `kelp.wgsl`: header comment with the abyssal-garden attribution (`W_LEAF`, `shaders.js:382-440`). Port of
  `vs` for one kelp type:
  - `v = sin(row / (ROWS - 1) * π/2)`; `t` = `u[0].z`; `len`, `s1`, `s2`, width and curvature from the
    instance (`i.c.w` → curvature, `i.b.z` → width).
  - Root: instance root with its x multiplied by `aspect / 1.6`.
  - Direction `d` = `normalize(vec3f(lean * aspect / 1.6 * (sin(PITCH) + cos(PITCH) * forward), 1.0, forward))`
    (keeps the blade's line at constant picture x). `pe` = the unit vector perpendicular to `d` in the plane
    of `d` and world up, pointing away from up (the blade droops as it bends); `sd = normalize(cross(d, pe))`.
  - Keep `k`, `wph`, `wv`, `kq/ka`, `sK/cK`, the centre line `c`, `sl`, `N`, `T0`, `tw`, `S`, `N2`, `shape`,
    `ruf` and `sway(...)` as in `W_LEAF`, with `nearK = 1`, `farK = 1`, no shadow-pass branch, `wav = 1.9`,
    `fls = 1.0`, `shp = vec2f(0.5, 0.32)` (type 0).
  - Swell push, after the sway: `p.x += SWELL_PUSH * pow(v, 1.5) * sin(u[0].y + sway_phase)` with
    `const SWELL_PUSH: f32 = 0.3;` (metres at the tip).
  - Outputs: world position, normal (as `W_LEAF`'s `o.n`), uv `(v, u)`, colour from type 0
    (`mix(vec3f(.11,.125,.022), vec3f(.04,.10,.04), s1)`, then `mix(.., vec3f(.25,.17,.025), step(.72, s2))`),
    and `lift = v * len`.
- Fragment: today's blade branch (ambient, sunlight, amber glow through the blade, midrib, wrinkles,
  `through_water`, root gloom via `lift`), with the instance colour in place of the fixed blade colour; drop
  the stalk branch and `BLADES`, `CURRENT`, `SURGE`.

- [ ] **Step 8: Run all tests, fmt, clippy**

Run: `cd scene && $HOME/.cargo/bin/cargo fmt && $HOME/.cargo/bin/cargo test --workspace -- --test-threads=1 && $HOME/.cargo/bin/cargo clippy --workspace --all-targets`
Expected: every test passes (including `kelp_is_drawn`, `shader_time_wraps_daily`, `same_frame_same_pixels`
and the luma checks), no warnings. If a fish- or luma-threshold check moves because the fish layout shifted
once, report it; don't loosen it without a ruling.

- [ ] **Step 9: Commit**

```bash
git add scene
git commit -m "feat(scene): kelp as a foreground frame of abyssal-garden blades"
```

### Task 2: Tune the look with the owner

**Files:**
- Modify: `scene/cove-render/shaders/kelp.wgsl` (colours, light constants, `SWELL_PUSH`)
- Modify: `scene/cove-scene/src/kelp.rs` (`KELP_PACE`, only if the owner asks)
- Modify: `docs/superpowers/specs/2026-11-16-kelp-vignette-design.md` (record the tuned values)

**Interfaces:**
- Consumes: Task 1's kelp pass and constants.

- [ ] **Step 1: Render the review set**

Run from `scene/`: snapshots at 960×600 for `--hour 7`, `12`, `17`, `22` (`--whales 0 --at 1763290000`), a
4:3 and a 21:9 snapshot at noon, and a 20 s `--frames` clip at noon encoded with ffmpeg; copy all to
`~/echomere-clips/kelp/` (the owner opens them from ChromeOS "Linux files").
Expected: blades at both edges only; dark against the surface; amber glow where the sun is behind them;
warm at 7 and 17; no bottle brushes; no blade cut by the bottom edge so that its end shows.

- [ ] **Step 2: Tune by eye**

Adjust colours, glow and `SWELL_PUSH` until the review set reads as the design describes; re-render. Keep
every test green.

- [ ] **Step 3: Owner review**

Show the owner the review set; apply their changes (including `KELP_PACE`). Record the final constants in the
spec's "Decided" section.

- [ ] **Step 4: Commit**

```bash
git add scene docs/superpowers/specs/2026-11-16-kelp-vignette-design.md
git commit -m "feat(scene): tune the kelp frame with the owner"
```
