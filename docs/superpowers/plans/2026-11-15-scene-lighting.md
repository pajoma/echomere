# Cove Scene Lighting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Light and fog every element of the Metal cove scene with one shared, physically shaped water model, ported from abyssal-garden.

**Architecture:** All new shading lives as Metal functions at the top of `SceneShaders.source` (after the `Sky` struct): constants, `toLinear`, `sunlightAt`, `ambientLight`, `inScatter`, `throughWater`, `finish`. Each fragment shader (`scene_fragment`, `kelp_fragment`, `whale_fragment`, `fish_fragment`) switches from its own constants to these and ends in `finish()`. Renderer, pass structure, uniforms and `Sky.swift` stay unchanged.

**Tech Stack:** Swift 5.9+, Metal Shading Language compiled at runtime, macOS 14+, Command Line Tools (no Xcode).

**Spec:** `docs/superpowers/specs/2026-11-15-scene-lighting-design.md`

## Global Constraints

- Branch `feature/7-scene-lighting` (off `feature/7-cove-scene`); PR targets `feature/7-cove-scene`, never `develop`/`main` directly.
- Budget: 15 fps at half resolution; no extra render pass, render target or loop; < ~10 % extra GPU time.
- Lighting depends only on the frame's inputs (time, sky, seed): `the same frame renders to the same pixels` must stay green.
- Starting constants: `SIGMA = (0.058, 0.0235, 0.0195)`, `KD = (0.056, 0.0245, 0.0170)`, `WATER_COLOUR = (0.16, 0.58, 0.90)`; distances: fish depth 0–1 → 3–25 m, echoes 20 m, snow 2 m / 6 m by layer; open water without a hit: 240 m.
- Linear light inside; `finish()` ends with `pow(c, 1/2.2)`; existing hand-picked colours go through `toLinear()` (`pow(c, 2.2)`); the whale skin is decoded after sampling.
- `Sky.swift` unchanged (night `ambient` 0.26 may be raised only if the night check fails).
- Ported functions carry `// Adapted from abyssal-garden, Copyright (c) 2026 Maksymilian Dendura, MIT License.`; `THIRD-PARTY-NOTICES.md` gets the abyssal-garden MIT licence.
- **Swift and Metal exist only on the owner's Mac.** Every `Run` step marked *(Mac)* is run by the owner; the implementer writes code, the owner pastes the output back. Never claim a *(Mac)* step passed without that output.
- `git add` named paths only (the local `abyssal-garden/` clone is untracked on this branch).

## Review Focus

1. Around 06:00 and 18:00 neither sun nor moon is up (`lightIntensity` ≈ 0) → picture must not go black. Pinned in Task 1 (`brightness at 06:00 and 18:00`).
2. Midnight under the moon → dark but visible. Pinned in Task 1.
3. Golden hour looking towards the low sun → warm glow, not washed out. Pinned in Task 1 (hour 7 within range).
4. `inScatter` divides by `SIGMA - KD·ray.y`, which crosses 0 in green for steep upward rays (ray.y ≈ 0.96) → no NaN/black pixels at the top edge. Pinned in Task 2 (guard) and Task 1 (`no black or NaN pixels`).
5. Shader compile errors surface only on the Mac → `the shaders compile` check already exists; Task 2 adds nothing that bypasses it.

---

## File Structure

| File | Change |
|---|---|
| `macos/EchomereScene/Sources/CoveSceneRenderer/SceneShaders.swift` | New water-model section; four fragment shaders use it |
| `macos/EchomereScene/Sources/SceneChecks/main.swift` | New "Lighting" checks in the Rendering block |
| `THIRD-PARTY-NOTICES.md` | abyssal-garden MIT licence |

---

### Task 1: Lighting checks

These checks guard behaviour the new model must keep; most already pass on today's shaders. The look itself is judged by the owner (Task 5), not by a check.

**Files:** Modify `macos/EchomereScene/Sources/SceneChecks/main.swift` (inside the `if let device…renderer` block, after the brightness check, ~line 314)

**Interfaces:**
- Consumes: `renderer.snapshot(_:width:height:) throws -> CGImage`, `frame` (line 247), `SceneFrame.sky` (var), `Sky.state(hour:)`.
- Produces: helper `func pixels(_ f: SceneFrame) -> [SIMD3<Double>]?` (320×200 snapshot, RGB 0…1, row-major from top; byte order B,G,R,A as in the existing brightness check), `func meanLuma(_ p: [SIMD3<Double>]) -> Double`.

- [ ] **Step 1: Write the checks**

```swift
print("Lighting")
func at(hour h: Double) -> SceneFrame { var f = frame; f.sky = Sky.state(hour: h); return f }
let byHour = Dictionary(uniqueKeysWithValues: [0.0, 6, 7, 12, 18].map { ($0, pixels(at(hour: $0))) })
check("noon is brighter than midnight", meanLuma(byHour[12]!!) > meanLuma(byHour[0]!!) * 1.5)
for h in [0.0, 6, 18] {
    check("brightness at \(Int(h)):00 is dark but not black (\(String(format: "%.3f", meanLuma(byHour[h]!!))))",
          meanLuma(byHour[h]!!) > 0.02)
}
check("golden hour is not washed out", meanLuma(byHour[7]!!) < 0.35)
let deep = byHour[12]!!.suffix(320 * 40)            // bottom fifth: looking into the deep
let avg = deep.reduce(SIMD3<Double>.zero, +) / Double(deep.count)
check("the deep is blue: blue > green > red (\(avg))", avg.z > avg.y && avg.y > avg.x)
check("no black or NaN pixels at noon", byHour[12]!!.allSatisfy { ($0.x + $0.y + $0.z) > 0.004 })
```

- [ ] **Step 2: Run** *(Mac)* `cd macos/EchomereScene && swift run SceneChecks` — Expected: compiles; record which Lighting checks pass on the old shaders (baseline, any result is acceptable here).
- [ ] **Step 3: Commit** — `git add macos/EchomereScene/Sources/SceneChecks/main.swift && git commit -m "test(scene): lighting checks for day, night and the deep"`

Deviation from spec goal 7: the "whale at 40 m vs 10 m" check is not automated — `WhalePose` has no public initialiser, and adding public surface only for a check is not worth it. It moves to the owner's visual check in Task 5.

---

### Task 2: The water model

**Files:** Modify `SceneShaders.swift` (new `// MARK: Water` section directly after `skyFrom`, ~line 77; remove `toneMap` at ~line 213 only in Task 3 when its last caller goes); modify `THIRD-PARTY-NOTICES.md`.

**Interfaces (Metal, all `float3` in linear light):**
- `constant float3 SIGMA, KD, WATER_COLOUR; constant float EXPOSURE = 0.6; constant float OPEN_WATER = 240.0; constant float3 CAMERA = float3(0.0, -CAMERA_DEPTH, 0.0);`
- `float3 toLinear(float3 c)` → `pow(max(c, 0.0), 2.2)`
- `float3 sunlightAt(float depth, Sky s)` → `toLinear(s.colour) * s.intensity * exp(-KD * depth / max(sunInWater(s.light).y, 0.2))`
- `float3 ambientLight(float depth, float3 normal, Sky s)` → port of abyssal `amb` (`shaders.js:259-262`) with `sky = 1`, `z = 0`, `p.y = -depth`, multiplied by `s.ambient`, plus `side` (`shaders.js:264`) with `open = 1`.
- `float3 inScatter(float3 origin, float3 ray, float distance, Sky s)` — body below.
- `float3 throughWater(float3 c, float3 origin, float3 ray, float distance, Sky s)` → `c * exp(-SIGMA * distance) + inScatter(origin, ray, distance, s)`
- `float3 finish(float3 c, float2 uv, float2 pixel)` → `c *= EXPOSURE`, ACES `tmap` + luminance S-curve + desaturation (port of `shaders.js:749-777`, without the shaft and bloom lines), then the existing vignette `1 - 0.35·dot(v,v)` with `v = (uv - 0.5)·(1, 0.8)`, then `pow(c, 1/2.2)`, then dither `(hash21(pixel) - 0.5) / 255`.

```metal
float3 inScatter(float3 origin, float3 ray, float distance, Sky s) {
    float3 a = SIGMA - KD * ray.y;
    a = select(a, float3(1e-4), abs(a) < 1e-4);              // Review Focus 4: no division by ~0
    float3 integral = (1.0 - exp(-a * distance)) / a;
    float mu = max(dot(ray, sunInWater(s.light)), 0.0);
    float3 light = WATER_COLOUR * s.ambient * 1.22                           // diffused light: never 0
                 + WATER_COLOUR * toLinear(s.colour) * s.intensity * 0.3 * pow(mu, 4.0);  // glow towards sun/moon
    return SIGMA * exp(KD * origin.y) * integral * light;
}
```

- [ ] **Step 1:** Add the section and functions above (attribution comment on `ambientLight`, `inScatter`, `finish`). Nothing calls them yet.
- [ ] **Step 2:** Append the abyssal-garden licence to `THIRD-PARTY-NOTICES.md` (copy `abyssal-garden/LICENSE` verbatim, source URL `https://github.com/noniv/abyssal-garden`).
- [ ] **Step 3: Run** *(Mac)* `swift run SceneChecks` — Expected: `the shaders compile` ok; all other results unchanged from Task 1's baseline.
- [ ] **Step 4: Commit** — `git commit -m "feat(scene): shared water model for lighting and fog"` (both files).

---

### Task 3: Open water, surface, shafts, echoes and snow

**Files:** Modify `SceneShaders.swift`: `water()`, `surfaceUnderside()`, `lightShafts()`, `drawEcho()`, marine snow loop and end of `scene_fragment`; delete `toneMap` and `water` once unused.

**Interfaces:** Consumes Task 2 functions. Changes:
- Open water (no surface hit, or `ray.y <= 0`): `inScatter(CAMERA, ray, OPEN_WATER, sky)` replaces `water(y, ambient)`.
- `surfaceUnderside`: sky through Snell's window as today but `toLinear(skyColour(...))` and no `toneMap`; mirrored part = `inScatter(hit, mirrored, OPEN_WATER, sky) * 1.15` (abyssal `shaders.js:604-605`); result through `throughWater(c, CAMERA, ray, t, sky)` instead of `clear`/`body`.
- `lightShafts`: keep the loop and `focus`; per step replace `exp(-depth * 0.07) * exp(-t * 0.05)` and `tint` with `sunlightAt(depth, sky) * exp(-SIGMA * t) * SIGMA * WATER_COLOUR`; keep `phase`, `reach`, offset; new overall factor `SHAFTS = 1.0` (tune in Task 5).
- Echoes and snow: their colours go through `toLinear`, then `throughWater` at 20 m (echo) and 2 m / 6 m (snow layers 0 / 1) along `ray`, and are added as the emitted part only (`c * exp(-SIGMA·d)`, no second `inScatter`).
- `scene_fragment` returns `float4(finish(col, in.uv, in.position.xy), 1.0)`; the vignette and dither lines there are removed (now in `finish`).
- `seaFromAbove` (debug view): ends in `finish()` too.

- [ ] **Step 1:** Make the changes above.
- [ ] **Step 2: Run** *(Mac)* `swift run SceneChecks` — Expected: all checks ok (Task 1 Lighting checks included). Then `swift run ScenePreview --hour 12` — Expected: background, surface and shafts in the new light; kelp, whales and fish still in the old light (clearly different from the background is expected at this step).
- [ ] **Step 3: Commit** — `git commit -m "feat(scene): open water, surface and light shafts use the water model"`

---

### Task 4: Kelp, whales and fish

**Files:** Modify `SceneShaders.swift`: `kelp_fragment`, `whale_fragment`, `drawFish`/`fish_fragment`; delete `fogged()` once unused.

**Interfaces:** Consumes Task 2. Sky in kelp/whale fragments is built as `Sky s; s.light = normalize(u[2].xyz); s.intensity = u[2].w; s.colour = u[3].rgb; s.ambient = u[3].w;` (whales: `w[4]`, `w[5]`); zenith/horizon unused there. Changes:
- Kelp: albedos through `toLinear`; `sunlight` → `sunlightAt(depth, s)`; the `0.25·ambient`/`0.30·ambient` terms → `ambientLight(depth, normal, s)`; fog block (`clear`, `water(y)`, root fade) → `throughWater(c, CAMERA, -view, distance, s)` with the existing root fade `smoothstep(0, 9, lift)` kept as a mix towards `inScatter(CAMERA, -view, distance, s)`; end with `finish(c, uv, in.position.xy)` where `uv = (in.position.x · u[1].x / u[0].x, 1 - in.position.y · u[1].x)`.
- Whales: `albedo = toLinear(skin.sample(...).rgb)`; the ambient term → `ambientLight(depth, normal, s)`; sunlight → `sunlightAt(depth, s) · saturate(dot(normal, sun)) · 2.2 · dapple`; fog → `throughWater`; end with `finish`; alpha stays `w[2].y`.
- Fish: `col` through `toLinear`, lit by `ambientLight(depthM, float3(0,1,0), sky) + sunlightAt(depthM, sky) · 0.3`, with `depthM = CAMERA_DEPTH - (distance · ray.y)` and `distance = mix(3.0, 25.0, a.z)`; fogged by `throughWater(col, CAMERA, viewRay(uv, aspect), distance, sky)`; `fish_fragment` applies `finish` to the premultiplied colour before returning (remove its own vignette).

- [ ] **Step 1:** Make the changes above.
- [ ] **Step 2: Run** *(Mac)* `swift run SceneChecks` — Expected: all ok, including `the same frame renders to the same pixels` and the brightness range check.
- [ ] **Step 3: Commit** — `git commit -m "feat(scene): kelp, whales and fish lit by the water model"`

---

### Task 5: Owner's look review and tuning

- [ ] **Step 1:** Owner runs *(Mac)* and sends the snapshots back:

```sh
for h in 0 6 7 12 18 21; do swift run ScenePreview --snapshot /tmp/light-$h.png --seed 7 --at 1794000500 --size 1440x900 --hour $h; done
swift run ScenePreview --day-seconds 72
```

  Visual pass criteria (owner judges): water reads as depth, not a gradient; deep is dark blue; the surface glows near the sun; a distant whale fades into the water more than a near one (replaces spec check 3); night dark but visible; no orange "aquarium" look; nothing brighter or more saturated than before overall.
- [ ] **Step 2:** Tune only named constants (`SIGMA`, `KD`, `WATER_COLOUR`, `EXPOSURE`, `SHAFTS`, fish/echo/snow distances) from the owner's feedback; one commit per round: `git commit -m "tune(scene): <what> after look review"`.
- [ ] **Step 3:** Push `feature/7-scene-lighting` and open a PR into `feature/7-cove-scene` (`gh pr create --base feature/7-cove-scene`), noting: ported MIT code (licence review), generated code (security review), snapshots attached by the owner.
