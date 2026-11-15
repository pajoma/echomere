# Cove scene lighting: one water model — Design

Part of issue #7 (the cove scene, PR #10). Step 1 of 2; step 2 (kelp look and motion) gets its own design.

## Intent

The cove scene must look like a real webcam 30 m under the sea, not a blue picture. The PR #10 notes call
the look "still stylised rather than truly realistic", and the owner named lighting as the first thing that
bothers them. The reference for "realistic" is abyssal-garden (noniv/abyssal-garden, MIT, local clone in
`abyssal-garden/`, gitignored): its water reads as real because every object is lit and fogged by the same
physically shaped water model.

What is wrong today (`SceneShaders.swift` on `feature/7-cove-scene`):

- The water colour is a gradient by screen height (`water(y, ambient)`), not light travelling through water.
- Each object has its own clarity constants: surface `(0.0150, 0.0130, 0.0100)`, kelp `(0.052, 0.040,
  0.036)`, whales `(0.034, 0.021, 0.017)`, fish a separate `fogged()` mix. Nothing shares one water.
- Sunlight fades with depth by one number (`exp(-depth * 0.055)`, whales `0.045`), so it never turns blue
  with depth.
- Ambient light ignores which way a surface faces, except a simple up/down term on the whales.
- Only the surface is tone-mapped (`1 - exp(-c)`); everything else is clamped with `saturate`.

## Goals

1. **One water model** in `SceneShaders.swift`, used by every element: surface underside, open water,
   light shafts, echoes, marine snow, kelp, whales, fish.
   - Per-channel extinction `SIGMA` and diffuse attenuation `KD` (red lost fastest, blue slowest).
     Starting values from abyssal-garden: `SIGMA = (0.058, 0.0235, 0.0195)`, `KD = (0.056, 0.0245,
     0.0170)` per metre. Named constants, tuned by eye on the Mac.
   - `inScatter(origin, ray, distance, sky)`: light scattered towards the camera along a line of sight,
     integrated in closed form with depth-dependent attenuation and a forward-scattering lobe towards the
     sun (port of abyssal-garden `fogIn`, `shaders.js:210-217`). It replaces `water(y, ambient)`: the open
     water, the deep below the horizon (darker by itself, because the light comes from above) and the
     background behind everything all come from it.
   - `throughWater(colour, distance, ray, sky)`: what reaches the camera from an object at `distance`:
     `colour · exp(-SIGMA · distance) + inScatter(...)`. Replaces all per-object `clear` constants and
     `fogged()`.
   - `sunlightAt(depth, sky)`: the sun's or moon's light at a depth, per channel:
     `sky.colour · sky.intensity · exp(-KD · depth / sunInWater.y)`. Replaces `exp(-depth · 0.055)` and
     `0.045`.
   - `ambientLight(depth, normal, sky)`: light from the water around, brighter from above, falling off with
     depth per channel, scaled by `sky.ambient` (port of abyssal-garden `amb`, `shaders.js:259-262`, plus a
     sideways fill like `side`, `shaders.js:264`).
2. **The water's colour belongs to the water model**: the `WATER_COLOUR` used by `inScatter` starts at
   abyssal-garden's `WCOL = (0.16, 0.58, 0.90)`, so the scene's blue comes out of physics instead of
   hand-picked gradient stops.
3. **One finishing step for every pixel**: a shared `finish(colour, pixel)` applied in every fragment
   shader: exposure, ACES tone map, a gentle luminance S-curve, the existing vignette and dither (port of
   abyssal-garden `W_POST`, `shaders.js:749-780`, without shafts and bloom). Replaces `toneMap` and the
   per-shader `saturate`.
4. **Day and night keep working**: everything scales with `Sky`'s `intensity`, `colour` and `ambient`, so
   noon, golden hour, moonlight and the dark hours before dawn each look plausible. Night stays dark but
   not black.
5. **The surface keeps its optics**: Snell's window, total internal reflection, Fresnel and the FFT waves
   stay as they are. Only the colour of what is reflected (today a hand-picked gradient) and the water
   between surface and camera go through the new model.
6. **Light shafts keep their algorithm** (24 steps along the ray, focus from wave curvature) but take their
   colour and falloff from `sunlightAt` and `SIGMA` instead of the hand-tuned `tint` and `exp(-depth ·
   0.07)`.
7. **Checks**: new `SceneChecks` rules on rendered snapshots (they need Metal, so they run on the Mac):
   - noon is brighter than midnight, and midnight is not black;
   - in the lower third (looking into the deep) blue > green > red;
   - a whale at 40 m is closer to the open-water colour than the same whale at 10 m.

## Non-goals

- Kelp placement, look and motion (step 2, separate design).
- Bloom, an HDR offscreen target or any extra full-screen pass.
- A rendered seabed: the camera stands on it but never sees it.
- Changes to the FFT ocean, wave optics, whale paths, fish behaviour or echoes' timing.
- Real local time per cove (Sky stays UTC for every cove).

## Anti-goals (a failure even if it works)

- Brighter, more saturated, "aquarium" or cartoon colours. It must stay dark, slow and calm.
- Noticeably more GPU time: the budget stays 15 fps at half resolution. The new model adds a few `exp`
  calls per pixel and no loops; anything costing more than about 10 % extra GPU time per frame is wrong.
- Two screens showing different pictures: lighting depends only on time and the cove seed (ADR 0005).
- A black screen at night.

## Constraints

- Metal Shading Language inside `SceneShaders.source`, compiled at runtime; macOS 14+, Command Line Tools
  only (no Xcode); checks are the `SceneChecks` executable.
- The scene draws straight into the layer's `bgra8Unorm` drawable in one render pass (full-screen pass,
  kelp, whales, fish with depth). That is why the finishing step runs in each fragment shader rather than
  in a separate pass.
- The author's machine has no Swift or Metal: the code can be written but not run there. Every visual
  judgement and the Metal checks happen on the owner's Mac with `swift run ScenePreview` and `swift run
  SceneChecks`.
- abyssal-garden code is MIT: ported functions carry the attribution comment and THIRD-PARTY-NOTICES.md
  gets its licence.

## Approaches

The order of the two improvements was the open choice:

- **A. Lighting first, then kelp (chosen by the owner, "lighting is prio").** One water model fixes every
  element at once; kelp shading in step 2 is then tuned once, against the right light.
- B. Kelp first: visible sooner, but kelp colours tuned against the old light would be redone.
- C. Both at once: fastest, but when the Mac preview looks wrong it is unclear which change caused it.

Within lighting, how to finish the picture:

- **Per-fragment finishing step (my call).** Each fragment shader calls `finish()`. No extra pass, no
  extra memory, nothing changes in the renderer's pass structure. Cost: fish blend after tone mapping
  (negligible for small, mostly opaque fish), and bloom stays impossible without a later change.
- HDR offscreen target plus a composite pass (as abyssal-garden does): enables bloom later, but adds a
  render target and a full-screen pass, against the energy goal, for a feature that is a non-goal.

## Decided by the owner

- Lighting is the priority; kelp follows in step 2.
- Kelp stays in the background, but closer than today so its movement can be seen (step 2).
- The reference for realism is abyssal-garden.

## Calls made by the designer (owner to check)

- **Linear light with a gamma encode at the end.** The model works in linear light and `finish()` ends
  with `pow(c, 1/2.2)`, as abyssal-garden does; the drawable stays `bgra8Unorm`. Every existing
  hand-picked colour (fish, echoes, marine snow, kelp, the colours in `Sky.swift`) is treated as
  display-encoded and decoded once with a shared `toLinear()` (`pow(c, 2.2)`) where it enters the shader,
  and the whale skin texture is decoded the same way after sampling. So the first build keeps today's
  colours, and only the light changes. If the picture comes out too light, the fix is exposure, not
  dropping the gamma encode.
- **What lights the open water:** the base term of `inScatter` is the light diffused by the water, scaled
  by `sky.ambient` and coloured by the water colour. The forward-scattering lobe towards the sun or moon
  is scaled by `sky.intensity` and coloured by `sky.colour`. This matches the physics, since underwater
  most light arrives diffused. It keeps the water lit around 06:00 and 18:00, when neither sun nor moon
  is up, and it lets golden hour warm the water only when looking towards the sun, not everywhere.
- **`Sky.swift` stays as it is.** Its values are inputs. Only if the "midnight is not black" check fails
  on the Mac may the night `ambient` key (0.26) be raised.
- **Distances for the 2D elements:** fish map their normalised depth `a.z` (0–1) to 3–25 m from the
  camera. Echoes and marine snow use fixed distances (echo 20 m, snow 2–6 m by layer). These are starting
  values to tune on the Mac.

## Left to the builder

- Exact constant values after the first Mac preview (starting points above), the exposure, and the
  S-curve strength.
- How `inScatter` is clamped where the ray never meets anything (abyssal-garden uses 240 m; the surface
  hit or a fixed distance both fit).
- Whether the reflected water colour under the surface comes from `inScatter` along the mirrored ray (as
  abyssal-garden does) or stays a simpler term, as long as it uses the same water colour.
