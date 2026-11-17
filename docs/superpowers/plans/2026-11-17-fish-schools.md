# Fish Schools Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the flat, rigid fish with two visiting schools (a big baitfish school and a smaller
mackerel school) that each screen simulates with the Couzin model and draws as swimming 3D fish.

**Architecture:** `cove-scene` computes, as a pure function of seed and time, which school visits are under
way and each visit's route (`SchoolVisit`). A new crate `cove-swarm` (CPU, stateful) simulates the fish of
those visits and returns `FishPose`s. The caller (preview, later the shells) owns the `Swarm`, puts its poses
into `SceneFrame.fish`, and `cove-render` draws them in a new instanced, depth-tested pass.

**Tech Stack:** Rust workspace `scene/`, wgpu, WGSL.

**Spec:** `docs/superpowers/specs/2026-11-17-fish-schools-design.md` (and ADR 0009).

## Global Constraints

- Shared between screens: only what `cove-scene` computes from (cove seed, visitors, echoes, time). The
  simulation may differ per screen (ADR 0009).
- World frame: x right, y up with the surface at 0, z ahead; camera at (0, −30, 0), pitched 40° up
  (`scene_rules`). "In the picture" = `camera::project(p, aspect)` gives x and y in 0..1.
- At most 512 fish instances.
- No platform `#[cfg]` in scene, swarm, render or shaders (ADR 0008). Dependencies: none new beyond the
  workspace's (`cove-swarm` uses only `cove-scene`).
- No third-party code copied; the Couzin model is implemented from Couzin et al. 2002 / 2005.
- Tests: `cd scene && $HOME/.cargo/bin/cargo test --workspace --no-fail-fast -- --test-threads=1`; fmt and
  clippy clean; commits end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. A screen that wakes after sleep (a gap of minutes or hours between frames): the school appears formed at
   its route's current place, not scattered and not from the visit's start — `swarm_restarts_after_a_gap`
   (Task 2).
2. A visit ending while fish are still in the picture: no school vanishes in view — routes end out of sight
   and `school_is_gone_only_out_of_sight` (Tasks 1, 2).
3. Both schools at once: no fish overlap across schools — `schools_do_not_overlap` (Task 2).
4. Screen shapes 9:16 to 21:9: entries and exits out of sight at both ends — `routes_enter_and_leave_out_of_sight`
   (Task 1).
5. Night: fish stay blue-dark, not red or glowing — `night_fish_are_not_red` (Task 3).

---

### Task 1: Shared school visits; old fish removed

**Files:**
- Create: `scene/cove-scene/src/schools.rs`; test `scene/cove-scene/tests/schools.rs`
- Modify: `scene/cove-scene/src/lib.rs`, `layout.rs` (drop `FishSchool`, `FishMember`, `schools`,
  `schools_for`), `scene.rs` (drop `CreaturePose`, `creatures`, `fish_poses`; add `visits`, `fish`)
- Modify: `scene/cove-render/src/water.rs` + `shaders/scene_buffer.wgsl` (drop the creature block; echoes move
  up; keep Rust and WGSL constants in sync), delete `src/fish.rs` and `shaders/fish.wgsl`, `gpu.rs` (no fish
  pass for now)
- Modify tests: remove `small_fish_*`, `fish_ignore_the_kelp` (kelp independence now trivially holds; keep
  `different_coves_have_different_kelp_and_fish` renamed `different_coves_have_different_kelp`),
  `fish_are_drawn`, `night_fish_are_not_red` (both return in Task 3); `scene-preview` print of creature count
  becomes visit count.

**Interfaces (produces, in `cove_scene::schools`):**
```rust
pub enum FishKind { Baitfish, Mackerel }          // Copy, Eq, Debug
pub struct Circling { pub start: f64, pub end: f64, pub centre: [f64; 3], pub radius: f64 }
pub struct SchoolVisit {
    pub id: u64,             // stable per visit (hash of seed, kind, slot)
    pub kind: FishKind,
    pub count: usize,        // Baitfish 300..=400, Mackerel 30..=60
    pub start: f64, pub end: f64,                   // seconds since 1970
    pub entry: [f64; 3], pub exit: [f64; 3],
    pub circling: Option<Circling>,                 // Baitfish only, in about half the visits
}
impl SchoolVisit { pub fn lead(&self, at: f64) -> [f64; 3]; }   // the route's lead point
pub fn visits(seed: CoveSeed, at: f64) -> Vec<SchoolVisit>;     // visits under way at `at`
pub const LEAD_SPEED: f64 = 0.9;                    // m/s along the route outside circling
```
`SceneFrame` gains `pub visits: Vec<SchoolVisit>` (filled by `CoveScene::frame`) and
`pub fish: Vec<FishPose>` (always empty from `CoveScene::frame`; the caller fills it), with
```rust
pub struct FishPose { pub position: [f64; 3], pub heading: [f64; 3], pub kind: FishKind,
                      pub size: f64, pub tail_phase: f64, pub speed: f64 }
```

**Schedule (designer's numbers):** time is cut into slots per kind, each slot holds one visit, from
`SplitMix64::new(combine(seed, kind, slot))`. Big school: slot 450 s, start = slot start + 0..135 s
(interval between starts 5¼–9¾ min). Small school: slot 300 s, start = slot start + 0..60 s (4–6 min). A
visit's duration follows from its route: route length / `LEAD_SPEED`, plus the circling time (Baitfish,
60–120 s). Routes: entry and exit at 12–30 m ahead along the view axis, at depth 8–24 m, sideways at
±2.2 × ahead (out of sight at 21:9), on opposite sides; the circle's centre in the picture's middle, radius
6–10 m. Small visits must end ≥ 30 s before the next small slot's earliest start (shorten the route's
depth change if needed); big visits are shorter than their slot.

Ruling to record: the spec's "small school every 2–4 minutes for 2–5 minutes" cannot hold without a school
overlapping itself; this plan uses 4–6 min between starts and a route-length duration (about 2–3 min),
so the small school is in the water about half the time, the big one about a quarter.

- [ ] **Step 1: Write the failing tests** in `cove-scene/tests/schools.rs`:
  - `visits_are_shared`: `visits(CoveSeed(5), t) == visits(CoveSeed(5), t)` for 50 times over a day; some
    `t` where `visits(CoveSeed(5), t) != visits(CoveSeed(6), t)`.
  - `big_school_visits_every_few_minutes`: over one day, starts of Baitfish visits 315..=585 s apart,
    durations 60..=300 s.
  - `small_school_visits_more_often`: Mackerel starts 240..=360 s apart, ≥ 30 s gap between visits, and the
    small school's share of the day in the water > the big school's.
  - `routes_enter_and_leave_out_of_sight`: for every visit of a day, `camera::project(lead(start))` and
    `lead(end)` are outside 0..1 in x at aspects 0.5625 and 2.333.
  - `lead_moves_smoothly`: `|lead(t + 1) − lead(t)| ≤ 1.5 × LEAD_SPEED` over each visit (sampled every 1 s),
    and `lead` stays ≥ 2 m below the surface and ≥ 10 m from the camera.
- [ ] **Step 2: Run** `cargo test -p cove-scene --test schools` → compile errors.
- [ ] **Step 3: Implement** `schools.rs`, the `SceneFrame` changes and the removals listed under Files.
- [ ] **Step 4: Run** the whole workspace suite → all pass (old fish tests removed as listed).
- [ ] **Step 5: Commit** `feat(scene): shared school visits; remove the flat fish`

### Task 2: `cove-swarm`, the Couzin simulation

**Files:** Create `scene/cove-swarm/` (`Cargo.toml` depending on `cove-scene` only, `src/lib.rs`), add to the
workspace; tests in `scene/cove-swarm/tests/swarm.rs`.

**Interfaces:**
- Consumes: `SchoolVisit`, `FishKind`, `FishPose` (Task 1).
- Produces:
```rust
pub struct Swarm { /* fish, per-school state, rng, last time */ }
impl Swarm {
    pub fn new(seed: u64) -> Self;
    /// Brings the swarm to `at` for the visits under way: spawns new visits' schools, removes ended ones,
    /// and integrates. A first call, a jump back in time or a gap over 0.25 s restarts every school
    /// mid-visit (spawned as a loose parallel group around `lead(at)`, heading along the route, then 3 s
    /// of warm-up).
    pub fn advance(&mut self, visits: &[SchoolVisit], at: f64);
    pub fn poses(&self) -> Vec<FishPose>;
}
pub fn polarisation(fish: &[FishPose]) -> f64;      // |mean unit heading|, 0..1
pub fn milling(fish: &[FishPose]) -> f64;           // |mean (r̂ × ĥ).y| around the school's centre, 0..1
```
**Model** (Couzin et al. 2002, 3D): per fish, zone of repulsion r_r (one body length), orientation
r_o, attraction r_a, blind angle behind, maximum turning rate per step, Gaussian angular noise, constant
cruise speed (Baitfish 1.1 m/s, Mackerel 1.3 m/s, both faster than `LEAD_SPEED`). Repulsion overrides; else
orientation + attraction. Informed fish (Couzin et al. 2005; about 10 % of a school) blend their desired
heading with the direction to `lead(at)` (weight ω ≈ 0.5). Circling: while a visit's circling window is
on, its school uses a narrow orientation zone (torus regime) and the lead point circles. Social rules only
within a school; repulsion between all fish. Fixed sub-steps of 1/30 s; neighbour search over a uniform grid
(cell = r_a). Fish keep ≥ 2 m below the surface and out of a 6 m sphere around the camera (steering away
like repulsion). `tail_phase` advances with speed / body length.

- [ ] **Step 1: Write the failing tests:**
  - `school_forms_parallel_group`: one Baitfish visit without circling, 20 s after start → polarisation > 0.8.
  - `school_mills_in_torus`: a visit inside its circling window, after 40 s → milling > 0.6, polarisation < 0.5.
  - `school_stays_together`: over 60 s, every fish within 25 m (Baitfish) of its school's centre.
  - `fish_keep_their_distance`: no two fish closer than half a body length, sampled every 1 s over 60 s.
  - `schools_do_not_overlap`: both kinds under way together → the same holds across schools.
  - `informed_fish_lead_the_school`: the school's centre stays within 12 m of `lead(at)` during the visit.
  - `school_starts_mid_visit` / `swarm_restarts_after_a_gap`: first `advance` in the middle of a visit, and
    a second `advance` 2 h later → polarisation > 0.8 and centre within 12 m of `lead`.
  - `school_is_gone_only_out_of_sight`: when a visit ends, its fish are removed, and at that moment none of
    them is in the picture at 16:10.
  - `simulation_fits_the_budget` (release only, `#[cfg_attr(debug_assertions, ignore)]`): `advance` by 1/15 s
    with 400 + 60 fish averages under 2 ms over 150 frames.
- [ ] **Step 2: Run** `cargo test -p cove-swarm` → compile errors.
- [ ] **Step 3: Implement** the crate.
- [ ] **Step 4: Run** `cargo test -p cove-swarm` and `cargo test -p cove-swarm --release` → pass; whole suite green.
- [ ] **Step 5: Commit** `feat(swarm): Couzin fish schools simulated on each screen`

### Task 3: Fish drawn as swimming 3D models

**Files:** Create `scene/cove-render/src/fish.rs`, `shaders/fish.wgsl` (new content), modify `gpu.rs`; tests
in `cove-render/tests/render.rs`.

**Interfaces:** consumes `SceneFrame.fish`. Instance per fish (stride 48): position + size, heading + speed,
kind + tail phase + 2 spare. Uniforms like the kelp's (aspect, time; 1/pixels high, warmth; light; colour +
ambient).

- Mesh per kind generated in code: a spindle body (length 1 along +x, about 12 rings × 8 segments) with a
  forked tail fin; Baitfish slimmer, Mackerel deeper. Scaled by `size` (Baitfish 0.15–0.2 m, Mackerel
  0.3–0.4 m).
- Vertex: body wave `z += A(x) · sin(k·x − tail_phase)`, A growing towards the tail; oriented by heading
  (banked slightly into turns not required); `project()`.
- Fragment: water-model lighting (`ambient_light`, `sunlight_at`, `through_water`, warmth); counter-shading
  (dark back, light belly); Baitfish: a silver glint, a mirror-like specular of the sun on the flank,
  strongest when the flank faces the light. Depth test `Less`, depth write, drawn after kelp and whales.

- [ ] **Step 1: Write the failing tests:**
  - `fish_are_drawn`: a frame with 40 `FishPose`s 15 m ahead in view differs from the frame without in
    ≥ 0.2 % of pixels.
  - `kelp_covers_fish_behind_it`: a fish placed directly behind a kelp blade (along the same line of sight,
    farther away) leaves the blade's pixels unchanged.
  - `night_fish_are_not_red`: at 0, 3 and 22 h the changed pixels' mean has b ≥ g ≥ r (as the old test).
  - `baitfish_glint`: at noon, a school turned with its flanks towards the sun has a brighter 99th luma
    percentile than the same school turned away.
- [ ] **Step 2: Run** them → fail.
- [ ] **Step 3: Implement** the pass.
- [ ] **Step 4: Run** the whole suite → green.
- [ ] **Step 5: Commit** `feat(render): fish drawn as swimming 3D models`

### Task 4: Preview wiring and tuning with the owner

**Files:** `scene/scene-preview/src/main.rs`, `Cargo.toml`.

- Window, `--frames` and `--snapshot` each own a `Swarm` (seeded from the cove seed), call
  `swarm.advance(&frame.visits, frame.time)` and set `frame.fish = swarm.poses()` before drawing.
  `--school big|small` adds a forced visit of that kind starting 5 s before the chosen moment (built with
  the same route rules, a public `schools::forced_visit(seed, kind, at) -> SchoolVisit`).
- [ ] **Step 1:** Render the review set to `~/echomere-clips/fish/`: 30 s clips at noon with `--school big`
  and `--school small`, one at 17:00, stills at 12/17/22 h.
- [ ] **Step 2:** Tune (Couzin parameters within the checks, speeds, glint, colours) by eye; keep tests green.
- [ ] **Step 3:** Ask the marine-biologist reviewer (`.claude/agents/marine-biologist.md`) to judge the
  behaviour from the code and stills; apply what holds.
- [ ] **Step 4:** Owner review; record the tuned values in the spec.
- [ ] **Step 5: Commit** `feat(preview): fish schools in the preview; tuned with the owner`
