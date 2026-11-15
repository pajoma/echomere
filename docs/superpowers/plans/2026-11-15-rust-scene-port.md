# Rust Cove Scene Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The cove scene as a Rust workspace (`scene/`): deterministic logic, a wgpu renderer with WGSL shaders and the new water lighting, and a preview tool. It runs on macOS and Linux.

**Architecture:**
- `cove-scene` is a pure port of the Swift `CoveScene` module.
- `cove-render` renders a `SceneFrame` into a half-resolution `rgba16float` target. The target holds:
  - the FFT ocean (compute passes);
  - one full-screen water pass;
  - kelp, whales and fish geometry.

  A composite pass then upscales to the output and applies `finish()`.
- `scene-preview` is a winit window and snapshot tool.
- The Swift package is the reference. Each task ports named Swift sources, and their behaviour is pinned by tests carried over from `SceneChecks`.

**Tech Stack:** Rust (stable, via rustup), wgpu, winit, pollster, image, clap, WGSL.

**Spec:** `docs/superpowers/specs/2026-11-15-rust-scene-port-design.md` (includes `docs/superpowers/specs/2026-11-15-scene-lighting-design.md`); ADR `docs/adr/0008-one-renderer-rust-wgpu.md`.

## Global Constraints

- **Branch and PR:** branch `feature/7-rust-port`. The PR targets `feature/7-cove-scene`. Use `git add` with named paths only, because `abyssal-garden/` is an untracked local clone.
- **Reference:** the Swift sources in `macos/EchomereScene/Sources/` define behaviour. Port names in Rust style:
  - `CoveScene.frame(at:visitors:echoes:)` becomes `CoveScene::frame(&self, at: f64, visitors: &[Visitor], echoes: &[Echo]) -> SceneFrame`;
  - times are `f64` seconds since 1970 instead of `Date`;
  - `SIMD3<Double>` becomes `[f64; 3]`.
- **Determinism:** no runtime randomness, no `HashMap` iteration order in output, no platform `#[cfg]` in the scene or shaders. Platform code lives only in surface creation.
- **Dependencies:** only `wgpu`, `winit`, `pollster`, `bytemuck`, `image` (png and jpeg features only), `clap`. Licences MIT/Apache-2.0. Pin exact versions in `Cargo.lock`, and commit it.
- **Lighting:** the design's starting constants are `SIGMA = (0.058, 0.0235, 0.0195)`, `KD = (0.056, 0.0245, 0.0170)` and `WATER_COLOUR = (0.16, 0.58, 0.90)`.
  - Exposure starts at 0.6. Open water without a hit is drawn at 240 m.
  - Starting distances: fish 3–25 m, echoes 20 m, snow 2 m and 6 m.
  - Light is linear inside. `finish()` runs in the composite pass.
  - Hand-picked colours and the whale skin go through `to_linear` (`pow 2.2`).
- **Energy:** 15 fps and half resolution by default, and nothing is drawn while the window is occluded.
- **Attribution:** ported shaders carry their attribution comment: web-ocean and abyssal-garden (Maksymilian Dendura, MIT), and Evan Wallace's WebGL Water (MIT). `THIRD-PARTY-NOTICES.md` lists all three.
- **Whale models:** they stay CC BY-NC 4.0 in their own folder with their `LICENSE.md` (ADR 0007).

## Review Focus

1. **No GPU in the container:** the renderer must use the software adapter (lavapipe) instead of panicking. Pinned in Task 4 (`renders_on_any_adapter`).
2. **Long-running time precision:** shader time must be wrapped to one day, as `SceneBuffer` does with `% 86_400`, so far-future times don't stutter. Pinned in Task 5 (`shader_time_wraps_daily`).
3. **Window resized, fullscreen or occluded:** the half-resolution target is rebuilt on resize, and drawing stops while occluded. Pinned in Task 8 by a manual check.
4. **Missing or corrupt whale asset files:** loading returns an error that names the file, not a panic in the render loop. Pinned in Task 7 (`missing_whale_file_is_an_error`).
5. **Times around 06:00 and 18:00, when neither sun nor moon is up:** the picture must not go black. Pinned in Task 5 (lighting tests).

---

## File Structure

```
scene/
  Cargo.toml                      workspace
  rust-toolchain.toml             pinned stable channel
  cove-scene/src/{lib,random,model,sky,sea,kelp,layout,scene,camera}.rs
  cove-scene/tests/{determinism,whales,pods,motion,echoes,scene,sea,sky}.rs
  cove-render/src/{lib,gpu,targets,ocean,water,kelp,whales,fish,composite}.rs
  cove-render/shaders/{ocean,common,water,kelp,whale,fish,composite}.wgsl
  cove-render/tests/render.rs
  scene-preview/src/main.rs
assets/whales/prepared/           moved from macos/.../Resources/Whales (with LICENSE.md)
```

---

### Task 1: Toolchain, workspace, random generator

**Files:** create `scene/Cargo.toml`, `scene/rust-toolchain.toml`, `scene/cove-scene/{Cargo.toml,src/lib.rs,src/random.rs}`, `scene/cove-scene/tests/random.rs`; modify `.gitignore` (add `scene/target/`).

**Interfaces:**
- Produces:
  - `pub struct SplitMix64` with `new(seed: u64)`, `next(&mut self) -> u64`, `unit() -> f64`, `range(lo, hi) -> f64`, `int(lo: i64, hi: i64) -> i64`, `angle() -> f64`;
  - `pub fn stable_hash(text: &str) -> u64` (FNV-1a);
  - `pub fn combine(a: u64, b: u64) -> u64`.

  These are exact ports of `Random.swift`, with wrapping arithmetic for `&+`/`&*`.

- [ ] **Step 1:** Install tools. These are the steps the owner approved:
  - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal`
  - `sudo apt-get install -y libwayland-dev libxkbcommon-dev pkg-config`

  Expected: `cargo --version` prints a version. Record that version in `rust-toolchain.toml` as `channel = "<x.y.z>"`.
- [ ] **Step 2: Write the failing tests** in `tests/random.rs`:

```rust
#[test] fn splitmix_matches_reference() {            // Vigna's reference vector
    let mut g = SplitMix64::new(1234567);
    assert_eq!(g.next(), 6457827717110365317);
    assert_eq!(g.next(), 3203168211198807973);
}
#[test] fn fnv1a_matches_reference() {
    assert_eq!(stable_hash(""), 0xcbf29ce484222325);
    assert_eq!(stable_hash("a"), 0xaf63dc4c8601ec8c);
}
#[test] fn unit_in_range() { let mut g = SplitMix64::new(9); for _ in 0..10_000 { let u = g.unit(); assert!((0.0..1.0).contains(&u)); } }
#[test] fn int_inclusive() { let mut g = SplitMix64::new(3); let v: Vec<i64> = (0..1000).map(|_| g.int(2, 4)).collect(); assert!(v.contains(&2) && v.contains(&4) && v.iter().all(|x| (2..=4).contains(x))); }
```

- [ ] **Step 3:** Run `cd scene && cargo test -p cove-scene`. Expected: it fails to compile because the items are missing.
- [ ] **Step 4:** Implement `random.rs`.
- [ ] **Step 5:** Run `cargo test -p cove-scene`. Expected: 4 passed.
- [ ] **Step 6: Commit:** `git add .gitignore scene/Cargo.toml scene/Cargo.lock scene/rust-toolchain.toml scene/cove-scene && git commit -m "feat(scene): Rust workspace and portable random generator"`

---

### Task 2: Model, sky, sea, kelp motion

**Files:** `cove-scene/src/{model,sky,sea,kelp}.rs`; tests `tests/{sky,sea}.rs`.

**Interfaces:**
- Consumes: Task 1.
- Produces:
  - from `Model.swift`: `CoveSeed(u64)`, `enum WhaleSpecies { SouthernRightWhale, Orca }` with `length()`, `breathing_cycle() -> (f64, f64)`, `travels_in_pods()`, `pace()`, `breaths() -> (i64, i64)`, `breath_interval()`; `WhaleLook { species, size }` with `length()`; `Visitor { creature_id: String, look, arrived_at: f64 }`; `Echo { creature_id, look, left_at: f64 }`; `mod scene_rules` with the same constants (`MAX_WHALES = 6`, `MAX_ECHOES = 5`, `ECHO_LIFETIME`, `WHALE_FADE_IN`, `WORLD_WIDTH`, `WORLD_CENTRE_X`, `CAMERA_DEPTH = 30.0`, `CAMERA_PITCH = 0.262`, `CAMERA_FOCAL = 0.714`);
  - from `Sky.swift`: `SkyState` (fields as in Swift, `[f64; 3]` vectors) plus `sky::hour(at: f64)`, `sky::state(hour: f64) -> SkyState`, `sky::state_at(at: f64)`;
  - from `Sea.swift`: `mod sea` constants plus `sea::time(at: f64) -> f64`;
  - from `Kelp.swift`: `KelpFrond`, `KelpMotion`, `kelp::surge_period()`, `kelp::motion(at: f64)`.

- [ ] **Step 1: Write the failing tests.** Port `SceneChecks/main.swift` lines 256–283 ("The sea", "The sky"; line 285 needs a frame and goes to Task 3) as `#[test]`s with the same names in snake_case and the same thresholds:
  - `sea_loops_seamlessly`, `every_cove_same_sea`;
  - `sun_at_noon_moon_at_midnight`, `night_darker_never_black`, `no_light_on_horizon`, `light_never_jumps` (< 0.05 per minute), `sun_and_moon_within_lens`.

  Add the kelp-motion checks from lines 242–244: `kelp_sways_with_swell` and `kelp_sways_same_everywhere`. Use the shared helpers in `tests/common/mod.rs`: `NOON = 1_794_000_000.0`, `HOUR`, `DAY`, `visitor()`, `echo()`, ported from main.swift lines 24–35.
- [ ] **Step 2:** Run `cargo test -p cove-scene`. Expected: compile failure.
- [ ] **Step 3:** Port the four modules.
- [ ] **Step 4:** Run `cargo test -p cove-scene`. Expected: all pass.
- [ ] **Step 5: Commit:** `feat(scene): model, sky, sea and kelp motion in Rust`

---

### Task 3: Layout and frame: whales, pods, fish, echoes

**Files:** `cove-scene/src/{layout,scene,camera}.rs`; tests `tests/{determinism,whales,pods,motion,echoes,scene}.rs`.

**Interfaces:**
- Consumes: Task 2.
- Produces:
  - `CoveLayout::new(seed)` with `kelp: Vec<KelpFrond>`, `schools: Vec<FishSchool>`;
  - `CoveScene::new(seed)` and `frame(...) -> SceneFrame`;
  - `SceneFrame { time, whales: Vec<WhalePose>, creatures: Vec<CreaturePose>, kelp, kelp_motion, echoes: Vec<EchoPose>, sea_time, pub sky: SkyState }`;
  - `WhalePose`, `CreaturePose`, `EchoPose` (fields as in Swift; `call: Option<f64>`);
  - `camera::project(point: [f64; 3], aspect: f64) -> Option<(f64, f64, f64)>`.

  All of these are ports of `CoveLayout.swift` and `CoveScene.swift`. Derive `PartialEq, Clone, Debug` on all of them.

- [ ] **Step 1: Write the failing tests.** Port every check from main.swift lines 44–254 except the two kelp-sway checks (Task 2), plus line 285 (`scene_uses_its_own_sky`), as `#[test]`s grouped by their `print` heading into the test files above, with identical thresholds. That's 37 checks, including the per-whale loop at lines 164–175 as two tests (`orca_never_jumps`, `right_whale_never_jumps`). Add one determinism test in `tests/determinism.rs`:

```rust
#[test] fn frame_is_reproducible_across_constructions() {
    let a = CoveScene::new(CoveSeed(4711)).frame(NOON, &guests(), &traces());
    let b = CoveScene::new(CoveSeed(4711)).frame(NOON, &guests(), &traces());
    assert_eq!(a, b);
}
```

  `guests()` and `traces()` mirror main.swift lines 36–42.
- [ ] **Step 2:** Run `cargo test -p cove-scene`. Expected: compile failure.
- [ ] **Step 3:** Port `CoveLayout.swift` and `CoveScene.swift` (including `SceneCamera.project`).
- [ ] **Step 4:** Run `cargo test -p cove-scene`. Expected: all pass. A check whose measured value drifts from the Swift output (printed in its name) by more than float noise is a porting bug. Debug it; don't loosen the threshold.
- [ ] **Step 5: Commit:** `feat(scene): cove layout and frames in Rust, all behaviour checks ported`

---

### Task 4: GPU setup, targets, ocean, composite

**Files:** `cove-render/{Cargo.toml,src/{lib,gpu,targets,ocean,composite}.rs,shaders/{ocean,common,composite}.wgsl}`, `cove-render/tests/render.rs`; modify `THIRD-PARTY-NOTICES.md`.

**Interfaces:**
- Consumes: `cove_scene::SceneFrame`, `sea::*`.
- Produces:
  - `pub struct Renderer`, with `Renderer::new_headless() -> Result<Self, RenderError>` (requests an adapter with `force_fallback_adapter: false`, retries with `true`, and accepts lavapipe) and `Renderer::new(instance, adapter, device, queue, surface_format)`;
  - `render(&mut self, frame: &SceneFrame, target: &wgpu::TextureView, size: (u32, u32))`;
  - `snapshot(&mut self, frame: &SceneFrame, width: u32, height: u32) -> Result<image::RgbaImage, RenderError>`;
  - `sea_statistics(&mut self, sea_time: f64) -> Result<Vec<SeaStatistics>, RenderError>`;
  - `SeaStatistics { cascade, height_rms, slope_rms, curvature_rms }`.
  - Internals: `Targets` (half-resolution `rgba16float` color plus `depth32float`, rebuilt when the size changes, scale 0.5 by default) and `composite.wgsl`, which does a bilinear upscale plus `finish()` as in the lighting design Task 2: exposure, ACES, S-curve, desaturation, vignette, `pow(1/2.2)`, dither.
- Ocean: `ocean.wgsl` ports `OceanShaders.swift`'s passes `ocean_init`, `ocean_evolve`, `ocean_fft_rows`, `ocean_fft_cols` and `ocean_curvature`. Start from web-ocean's `js/wgsl/sim.js` and apply the Echomere changes listed in `OceanShaders.swift`'s header: 3 cascades, N = 256, no foam, a curvature pass, and the spectrum parameters from `sea.rs`.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test] fn renders_on_any_adapter() { assert!(Renderer::new_headless().is_ok()); }
#[test] fn sea_has_rough_open_sea_waves() {
    let s = Renderer::new_headless().unwrap().sea_statistics(123.0).unwrap();
    let total = s.iter().map(|c| c.height_rms * c.height_rms).sum::<f64>().sqrt();
    assert!(total > 0.5 && total < 2.0, "height RMS {total}");
}
```

- [ ] **Step 2:** Run `cargo test -p cove-render`. Expected: compile failure.
- [ ] **Step 3:** Implement. Run `cargo test -p cove-render`. Expected: 2 passed on lavapipe. Add the web-ocean and Evan Wallace attributions and the abyssal-garden licence to `THIRD-PARTY-NOTICES.md`.
- [ ] **Step 4: Commit:** `feat(render): wgpu setup, half-resolution target, FFT ocean and composite`

---

### Task 5: Water pass with the new lighting

**Files:** `cove-render/{src/water.rs,shaders/water.wgsl}`; extend `shaders/common.wgsl` and `tests/render.rs`.

**Interfaces:**
- Consumes: Task 4's ocean textures and `Targets`.
- Produces in `common.wgsl` the lighting design's functions, all in linear light: `to_linear`, `sunlight_at(depth, sky)`, `ambient_light(depth, normal, sky)`, `in_scatter(origin, ray, distance, sky)` (including the guard `a = select(a, vec3(1e-4), abs(a) < vec3(1e-4))`) and `through_water(c, origin, ray, distance, sky)`. Their bodies follow lighting plan Task 2 (`docs/superpowers/plans/2026-11-15-scene-lighting.md`, superseded, for the formulas).
- `water.wgsl` holds the full-screen pass. It ports `scene_fragment`, `surfaceUnderside`, `lightShafts`, `drawEcho` and the marine snow from `SceneShaders.swift`, with the changes from lighting plan Task 3: open water via `in_scatter` at 240 m, the surface through `through_water`, shafts from `sunlight_at`, and echoes and snow at fixed distances. There's no `finish` here, because the composite pass does it.
- The uniform buffer packs what `SceneBuffer.swift` packs. Shader time is `frame.time % 86_400.0`.

- [ ] **Step 1: Write the failing tests** in `render.rs`. They're ports of main.swift lines 301–314 plus the lighting checks from lighting plan Task 1, at 320×200:

```rust
#[test] fn same_frame_same_pixels()            // two snapshots equal
#[test] fn neither_black_nor_washed_out()      // mean luma in (0.05, 0.35), weights .2126/.7152/.0722
#[test] fn noon_brighter_than_midnight()       // luma(12h) > 1.5 * luma(0h), via frame.sky = sky::state(h)
#[test] fn dark_but_not_black_at_0_6_18()      // luma > 0.02 each
#[test] fn golden_hour_not_washed_out()        // luma(7h) < 0.35
#[test] fn deep_is_blue()                      // bottom fifth at noon: mean b > g > r
#[test] fn no_black_or_nan_pixels_at_noon()    // every pixel r+g+b > 1/255
#[test] fn shader_time_wraps_daily()           // frame at NOON and at NOON + 400 * DAY (same time of day): equal snapshots
```

- [ ] **Step 2:** Run `cargo test -p cove-render`. Expected: the new tests fail.
- [ ] **Step 3:** Implement the pass.
- [ ] **Step 4:** Run `cargo test -p cove-render`. Expected: all pass. Save `cargo run -p scene-preview` snapshots later, in Task 8.
- [ ] **Step 5: Commit:** `feat(render): open water, surface, shafts, echoes and snow lit by the water model`

---

### Task 6: Kelp

**Files:** `cove-render/{src/kelp.rs,shaders/kelp.wgsl}`; extend `tests/render.rs`.

**Interfaces:**
- Consumes: `SceneFrame.kelp`, `kelp_motion`, the ocean displacement texture, and `common.wgsl`.
- Port: `KelpMesh.swift` (the same vertex layout `(s, u, v, blade)`, 48 stalk segments, 64 blades, 9 rows, `u16` indices) and `kelp_vertex` with `stalkPoint`, unchanged. `kelp_fragment` gets lighting plan Task 4's changes. Instance data is two `vec4`s per frond, as in `CoveSceneRenderer.swift` lines 219–223.

- [ ] **Step 1: Write the failing test:** `kelp_is_drawn`. Render frame A (`munich_bay` at noon) once with kelp and once with `frame.kelp.clear()`. At least 0.1 % of pixels differ by more than 2/255.
- [ ] **Step 2:** Run it. Expected: FAIL (no kelp pass yet).
- [ ] **Step 3:** Implement. Run `cargo test -p cove-render`. Expected: all pass.
- [ ] **Step 4: Commit:** `feat(render): giant kelp geometry lit by the water model`

---

### Task 7: Whales and fish

**Files:**
- Move: `macos/EchomereScene/Sources/CoveSceneRenderer/Resources/Whales/*` to `assets/whales/prepared/` with `git mv`, and update `tools/prepare-whales.py`'s output path and `assets/whales/README.md`.
- Create: `cove-render/{src/whales.rs,src/fish.rs,shaders/whale.wgsl,shaders/fish.wgsl}`.

**Interfaces:**
- `WhaleModel::load(dir: &Path, species: WhaleSpecies, device, queue) -> Result<WhaleModel, RenderError>`. It reads `<name>.mesh`, `<name>.animation` and `<name>.jpg` in the formats defined in `tools/prepare-whales.py`, and ports `WhaleModel.swift`, including `pose(phase, effort)`. The asset directory is resolved at runtime: the `ECHOMERE_ASSETS` env var, otherwise `<exe dir>/assets`, otherwise the workspace's `../assets`.
- `whale.wgsl` ports `whale_vertex` and `whale_fragment` with lighting plan Task 4's changes. The skin is decoded with `to_linear`.
- `fish.wgsl` ports `fish_fragment` and `drawFish` with lighting plan Task 4's changes.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test] fn missing_whale_file_is_an_error() {
    let err = WhaleModel::load(Path::new("/nonexistent"), WhaleSpecies::Orca, &dev, &q).unwrap_err();
    assert!(err.to_string().contains("orca.mesh"));
}
#[test] fn whales_are_drawn()   // a frame with 3 visitors vs none at a time they are in view: ≥ 0.1 % pixels differ
#[test] fn fish_are_drawn()     // frame vs frame with creatures cleared: ≥ 0.05 % pixels differ
```

- [ ] **Step 2:** Run them. Expected: FAIL.
- [ ] **Step 3:** Implement. Run `cargo test` for the whole workspace. Expected: all pass.
- [ ] **Step 4: Commit:** `feat(render): whales and fish; whale assets shared by all platforms`

---

### Task 8: Preview tool

**Files:** `scene-preview/{Cargo.toml,src/main.rs}`.

**Interfaces:** `clap` options exactly as in `ScenePreview/main.swift` lines 51–64 (`--seed`, `--whales`, `--echoes`, `--speed`, `--fps` (default 15), `--scale` (default 0.5), `--view above`, `--hour`, `--day-seconds`, `--snapshot`, `--frames`, `--seconds`, `--at`, `--size WxH`), plus `--stats`, which prints GPU ms per frame from timestamp queries when the adapter supports them. The sample cove ports `SampleCove.swift`. Window behaviour:
- winit with a `ControlFlow::WaitUntil` frame pacing;
- Space shows another cove, Esc quits;
- resize rebuilds the targets;
- `WindowEvent::Occluded(true)` stops drawing.

- [ ] **Step 1:** Run `cargo run -p scene-preview -- --snapshot /tmp/cove.png --seed 7 --at 1794000500 --size 1440x900`. Expected: PNG written (headless path, lavapipe).
- [ ] **Step 2:** Implement, then repeat Step 1. Expected: the PNG exists. Look at it.
- [ ] **Step 3: Manual check (Review Focus 3).** Run `cargo run -p scene-preview -- --day-seconds 72` here, plus `--fps` and `--stats`. Resize, fullscreen and minimise the window, and confirm that the picture stays sharp and that minimising stops the frame counter.
- [ ] **Step 4:** Tune the first round of constants from snapshots at hours 0, 6, 7, 12, 18 and 21, against the visual criteria in lighting plan Task 5. Make one commit per round: `tune(scene): ...`.
- [ ] **Step 5: Commit:** `feat(preview): scene preview window and snapshot tool`

---

### Task 9: Owner review, energy, PR

- [ ] **Step 1:** The owner runs the preview on the Mac (`cargo run --release -p scene-preview`) and on Linux, and judges the look (issue #7 "done when"). Tuning feedback leads to more `tune(scene)` commits.
- [ ] **Step 2:** The owner compares Energy Impact in Activity Monitor between the Swift (`swift run ScenePreview`) and the Rust preview at the same window size and 15 fps. Rust may be at most about 10 % higher.
- [ ] **Step 3:** Push `feature/7-rust-port` and open a PR into `feature/7-cove-scene`: `gh pr create --base feature/7-cove-scene`. Note in it: ported MIT code (licence review), generated code (security review), the CC BY-NC assets unchanged, and the owner's snapshots.
- [ ] **Step 4:** Only after the owner approves the look and the energy comparison passes, `git rm -r macos/EchomereScene` in one commit: `chore: remove the Swift scene, replaced by scene/`.
