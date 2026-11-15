# Cove scene in Rust and wgpu, for macOS and Linux — Design

Part of issue #7 (the cove scene). Implements ADR 0008, which supersedes ADR 0002.

## Intent

Echomere must run on Linux as a real platform, not only on macOS. The owner chose one renderer for both:
Rust, wgpu and WGSL. The Swift/Metal scene on `feature/7-cove-scene` (PR #10) is the reference for how the
cove behaves (whale paths, breathing, pods, fish, echoes, sky, sea) and how it is framed (webcam on the
seabed, 30 m down, tilted 15° up). Its look is not the reference: the owner wants more realism, and lighting
comes first (`2026-11-15-scene-lighting-design.md`). So the port is the moment the new lighting goes in.

## Goals

1. **Cargo workspace `scene/`** at the repository root with four members:
   - `cove-scene` (library, no GPU): the deterministic scene, ported from `macos/EchomereScene/Sources/
     CoveScene/` (`CoveScene`, `CoveLayout`, `Model`, `Sky`, `Sea`, `Kelp`, `Random`). Same rule as
     ADR 0005: a frame is a pure function of (cove seed, visitors, echoes, time).
   - `cove-render` (library): draws a frame with wgpu into a window surface or an offscreen texture.
     Shaders in `.wgsl` files, included at compile time.
   - `scene-preview` (binary): a window and a snapshot tool with the options of today's `ScenePreview`
     (`--speed`, `--snapshot`, `--seed`, `--at`, `--size`, `--view above`, `--hour`, `--day-seconds`,
     `--whales`, `--echoes`, `--fps`, `--scale`).
   - Checks are `cargo test`: logic tests in `cove-scene`, render tests in `cove-render` (offscreen,
     headless). They replace `SceneChecks` one to one, plus the lighting checks of the lighting design.
2. **Same behaviour as the reference**: every check in today's `SceneChecks/main.swift` exists as a test and
   passes, with the same thresholds.
3. **Renderer parts**, each from its reference:
   - Sea: the FFT ocean as WGSL compute passes. Port from the original WGSL of web-ocean
     (github.com/Noniv/web-ocean, `js/wgsl/sim.js`, MIT) with Echomere's changes from `OceanShaders.swift`:
     3 cascades, size 256, no foam, the extra curvature pass.
   - Full-screen pass: surface underside with Snell's window, light shafts, echoes and marine snow, as in
     `SceneShaders.swift`, but lit by the water model.
   - Kelp: today's giant-kelp geometry and motion (`KelpMesh`, `kelp_vertex`) unchanged for now; the kelp
     redesign is step 2 with its own design.
   - Whales: skinned meshes from the existing prepared files (`.mesh`, `.animation`, `.jpg`, formats in
     `tools/prepare-whales.py`), moved to `assets/whales/prepared/` with their `LICENSE.md` (ADR 0007).
   - Fish: as today.
   - Lighting: the water model as in the lighting design (one model for every element, linear light, the
     designer's calls listed there), except that `finish()` runs in the composite pass (see the calls below).
4. **Runs on both platforms**: `cargo run -p scene-preview` opens the scene on macOS 14+ (Metal backend) and
   on Linux under Wayland and X11 (Vulkan, falling back to OpenGL). On this ChromeOS container it runs with
   "Linux GPU support" enabled, and without it on Mesa's software renderer (slow, but it runs).
5. **Energy as before**: 15 fps, half resolution (offscreen target plus one composite pass, see the calls
   below), no drawing while the window is hidden or occluded.

## Non-goals

- The platform shells: the macOS `.saver` (#9) and a Linux idle app (new issue). They come after the port.
- The kelp redesign (step 2), the creator's overlay (#8), the web page, the backend.
- A browser build for ChromeOS (possible later, wgpu supports WebGPU).
- Matching the Swift version's random numbers or pixels. Determinism means every screen running the Rust
  version computes the same frame.

## Anti-goals

- A behaviour check from today's `SceneChecks` silently dropped or loosened.
- More GPU time per frame than the Metal version at the same resolution (about 10 % slack for the new
  lighting).
- A scene that differs between two screens for the same inputs.
- Platform `#[cfg]` branches inside the scene or the shaders. Platform code lives only in the shells and in
  surface creation.

## Constraints

- MIT for code; whale models CC BY-NC 4.0 in their own folder (ADR 0007).
- Ported code keeps its attribution: web-ocean and abyssal-garden (Maksymilian Dendura, MIT), Evan
  Wallace's WebGL Water optics (MIT), as in today's `THIRD-PARTY-NOTICES.md`.
- No GPU in this container until "Linux GPU support" is enabled; render tests must also pass on a software
  adapter (wgpu's fallback adapter or Mesa lavapipe/llvmpipe).
- Dependencies limited to what the job needs: `wgpu`, `winit`, `pollster` (or equivalent), an image crate
  for the skin texture and snapshot PNGs, an argument parser. Licences MIT/Apache-2.0 only (no GPL).

## Approaches

Decided by the owner:

- **One renderer in Rust + wgpu + WGSL (chosen)** over two native renderers (every look change made twice)
  and over Metal plus a web scene for Linux (two renderers, and ADR 0002's energy argument against web
  views).
- **Rust over Swift (chosen)**: wgpu is native to Rust; Swift would need hand-made bindings. Cost: the
  770 lines of scene logic are ported, and the Swift work becomes reference.

Designer's call, for the owner to check:

- **Port with the new lighting directly**, not first a one-to-one port of today's look. The old look is
  what the owner wants to replace, so reproducing it first would be built only to be thrown away. Behaviour
  checks and framing still have to match the reference exactly; colours don't.
- **The Swift package stays in the repository until the Rust scene passes every check**, then it is deleted
  in one commit (history keeps it). Two more conditions before that commit: the owner has approved the look
  (issue #7's "done when"), and the energy comparison below shows no regression.
- **Half resolution through an offscreen target on every platform.** Linux surfaces don't scale up a smaller
  drawable the way a macOS layer does. So the scene renders into a half-resolution `rgba16float` texture,
  and one composite pass scales it to the window. That pass also does the lighting design's `finish()`
  (exposure, tone map, S-curve, vignette, gamma, dither) once per pixel, instead of in every fragment shader.
  This replaces the lighting design's "per-fragment finish, no extra pass" call. The new pass is one texture
  read per screen pixel and costs less than the half-resolution scene saves. There's one code path, with no
  platform branches.
- **Energy is compared by hand, not by a harness.** The owner runs the Swift and the Rust preview side by
  side on the Mac at the same window size and 15 fps, and compares "Energy Impact" in Activity Monitor. The
  Rust value must not be clearly higher (about 10 % slack). `scene-preview --stats` prints the GPU time per
  frame from wgpu timestamp queries where the adapter supports them, to help find the cause if it is.
- **Tuning the look:** the first round of constants is tuned here from snapshots rendered on the software
  adapter. The owner then judges and signs off on the Mac (and on Linux once the GPU is enabled). Software
  and hardware rendering produce the same colours within rounding, so the first round isn't wasted.
- **Determinism means "the same to the eye", not bit-identical.** The scene is a closed-form function of the
  time. Nothing accumulates from frame to frame, so last-digit differences between the macOS and Linux maths
  libraries stay at about 1e-12 m and never grow. No pure-Rust maths library is needed. The tests pin
  determinism on one machine (same inputs, same frame) and use tolerances where they compare numbers.

## Left to the builder

- Crate versions (current stable `wgpu` and `winit`), module layout inside the crates, error types.
- How render tests pick an adapter (hardware if present, otherwise fallback) and image comparison helpers.
- Whether the snapshot tool and the window share one binary with flags (as today) or two.
