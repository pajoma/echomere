# Kelp as a foreground frame — Design

Part of issue #7 (the cove scene). Step 3 of the owner's priority order for the scene's look: waves,
lighting, **kelp**, fish swarms, creatures. Replaces the kelp geometry ported from the Swift scene
(`KelpMesh.swift`, `kelp_vertex`), which the port design kept "unchanged for now".

## Intent

Echomere's cove is a meditative underwater webcam: the camera sits 30 m down and looks 40° up at the
surface, with no ground in the picture. Today's kelp is 5–7 giant-kelp plants, 55–100 m away and spread
across the whole width: thin stalks with small blades that read as dark bottle brushes. The owner wants
kelp as a **frame**: a few long blades rising in the foreground at the left and right edges of the picture,
like a vignette, moving with the blade animation of abyssal-garden (`W_LEAF`, which the owner liked from the
start). The middle of the picture stays open for the water, the light and, later, the whales.

## Goals

1. **A light frame (owner's choice "A")**: on each side a few long ribbon blades, covering roughly the
   outer 10 % of the picture width per side. They grow in from below the bottom edge and reach up along the
   side. The middle stays free.
2. **abyssal-garden's blade motion**: each blade is a ribbon that bends along its length (curvature), with a
   travelling wave down its length, a twist, rippling (ruffled) edges, and a gentle sway made of a slow
   motion and a smaller, faster one on top. Ported from `W_LEAF` (`abyssal-garden/source/src/shaders.js`,
   lines 382–440: `sway`, `shape2`, the vertex shader `vs`), with its MIT attribution.
3. **Meditative pace**: all of that motion runs slower than in abyssal-garden, in keeping with the slowed
   sea (`sea::SEA_PACE` = 0.25). Nothing in the frame should flick or flutter.
4. **The water model's light**: blades are lit like everything else (`common.wgsl`): dark silhouettes
   against the bright surface with light glowing through the thin parts where the sun is behind them; the
   red shift at sunrise and sunset (`warmth`) applies; through-water haze between blade and camera.
5. **Same frame on every screen of a cove, different per cove**: the blades are part of the cove layout
   (seeded, ADR 0005); their motion is a pure function of time.

## Non-goals

- Fish: they are drawn by a full-screen pass over everything (`fish.wgsl`, depth "always"), so they will
  swim in front of the kelp. That stays as it is; fish swarms are the next step.
- Whales (parked), echoes, the surface, the light shafts.
- Gas bladders, holdfasts, stalks: the frame is blades only.

## Anti-goals

- Kelp in the middle of the picture, or a frame so dense it reads as a wall (that was option "B").
- Hectic motion: a fast flutter, as today's 1.6 s `FLUTTER_PERIOD`, or blades whipping.
- Ground, roots or a seabed in the picture: the blades enter from below the bottom edge.
- A blade cut open by the camera's near plane (0.3 m) or ending abruptly in mid-picture.
- The old bottle-brush look.

## Constraints

- Rust + wgpu + WGSL (ADR 0008); no platform branches in the scene or shaders.
- Determinism: frame = f(seed, visitors, echoes, time). Two screens of one cove show the same kelp.
- Energy: the kelp pass must not cost more GPU time than today's kelp pass (16 fronds × 64 blades × 9 rows).
- Licences: abyssal-garden is MIT (Maksymilian Dendura); keep the attribution in the shader header and in
  `THIRD-PARTY-NOTICES.md` (which already lists abyssal-garden).
- Render tests run on lavapipe (`cargo test --workspace -- --test-threads=1`).

## Approaches

Owner picked **1**.

1. **Real 3D ribbon blades, `W_LEAF` ported (chosen).** The existing kelp pass (instanced mesh, depth test)
   stays; its mesh becomes one ribbon strip and its vertex shader becomes the `W_LEAF` blade. Blades get the
   water model's lighting, backlight and warmth for free. Cost: replaces the kelp layout, geometry, shader
   and checks.
2. Flat blades painted onto the picture edges in a screen-space pass: cheapest, but loses the twist and
   depth that make the abyssal-garden blades look good ("stickers on the glass").
3. Today's giant kelp moved close to the edges: little code, but keeps the bottle-brush look the owner
   doesn't like and lacks the blade motion the owner liked.

## Decided

By the owner:

- Kelp is a foreground frame at the left and right edges, light (about 10 % per side), not a dense wall.
- The motion is abyssal-garden's blade animation.
- Meditative: slow, calm.
- Approach 1.

Designer's calls, for the owner to check:

- **The background kelp goes.** The frame replaces it; nothing of today's giant kelp stays.
- **Placement**: blades are rooted below the picture's bottom edge (below the camera, about 31–36 m deep),
  2–8 m ahead of the camera, and lean outwards/upwards so they climb the side of the picture. Their lengths
  vary (about 8–20 m) so the frame has a ragged top, not a line; a few reach high, most stay in the lower two
  thirds of the side.
- **Count**: 3–6 blades per side, varying per cove seed; the two sides are not mirror images.
- **Screen shapes**: the picture's width depends on the screen (4:3 to 21:9). The blades' roots are placed
  for the 16:10 reference picture, and the vertex shader scales each root's sideways position by
  `aspect / 1.6`, so the blades keep to the edges on every screen. Only the roots move; the blades
  themselves are not stretched.
- **Swell**: on top of the `W_LEAF` sway, the whole frame is pushed gently back and forth by the swell
  (today's `surge`, period from the swell, now about 44 s), so the kelp keeps time with the waves above it.
  The old `drift` and `flutter` cycles go.
- **Pace**: the `W_LEAF` time terms run on a kelp clock of real time × `KELP_PACE`, starting at 0.4 and tuned
  by eye with the owner (abyssal-garden's blade wave `t * 1.1` and sway `t * .5` would otherwise read as
  busy). Like `SEA_PACE`, it is one constant in `cove-scene`.
- **Look**: one kelp type, olive to brown (abyssal-garden's kelp colours, type 0 and 4, as a starting
  point), with abyssal-garden's per-blade colour variation and translucency; no sparkle, no iridescence, no
  emissive types. The fragment keeps today's lighting recipe (ambient, sunlight, amber glow through the
  blade towards the camera, midrib and wrinkles, then `through_water`), tuned for blades seen up close.
- **Separate random stream**: the kelp blades are drawn from their own seeded stream (derived from the cove
  seed), so the fish schools' layout is independent of the kelp from now on. Removing the old kelp draws
  shifts every cove's fish layout once; that is fine (matching earlier random numbers was never a goal, port
  design "Non-goals"). No compatibility shim.
- **Measuring the frame**: placement and the 10 % are about the picture, not world x. A root is placed by
  its angle off the camera's forward direction (the 40° pitched view axis): its sideways offset is
  `tan(angle) × distance along the view axis`, so it lands at the intended picture x whatever its distance.
  Resting blades aim at the outer 10–12 % per side; the 20 % band in the check is the margin for sway and
  swell, and the check holds at every moment, sampled at several times over a swell period.
- **Swell push**: a small push, camera-relative and mostly sideways (along the picture's x), growing towards
  the blade tip: the tip of a blade 5 m away moves at most about 0.3 m (about 2–3 % of the picture width),
  so the swell is felt, never seen as a jerk.
- **Energy budget**: no more vertices than today's kelp mesh (16 × 64 × 9 rows, about 28 000 vertices), and
  blades are narrow ribbons covering only the edge bands, so the shaded pixel count stays small. The owner's
  Activity Monitor comparison on the Mac (port plan Task 9) is the real measurement; lavapipe timings are
  not.

## Checks (replace today's kelp checks)

- `kelp_frames_the_picture` (cove-scene): every blade's root is below the picture (below its bottom edge at
  the 16:10 reference) and close (2–8 m ahead); both sides have 3–6 blades.
- `kelp_keeps_the_middle_free` (render): with and without kelp, the pixels kelp changes lie almost all
  (≥ 95 %) in the outer 20 % of the width on each side, and both sides have some (each ≥ 2 % of its 20 %
  band). Checked at 16:10 and at 4:3, at several moments over a swell period.
- `kelp_sways_with_swell` and `kelp_sways_same_everywhere` stay (surge period about 44 s; same motion in
  every cove).
- `kelp_is_drawn` stays.
- `different_coves_have_different_kelp_and_fish` stays.
- Removed: `kelp_grows_where_giant_kelp_grows`, `kelp_is_only_hinted_at`.

## Left to the builder

- The data layout of a blade instance (root, heading, length, width, curvature, seeds) and the number of
  rows along a blade (enough that a blade seen at 2 m stays smooth).
- How abyssal-garden's `ROWS`, `FAR`/`nearK` distance fades and `pr` per-type table are reduced to the one
  kelp type.
- How the blade's root is hidden: below the bottom edge, or faded into the gloom as today's `lift` does.
- The exact colours and light constants, tuned from snapshots at noon, 17:00 and night.
