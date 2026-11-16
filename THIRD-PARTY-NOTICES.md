# Third-party notices

Echomere's code is under the MIT licence (see `LICENSE`). Parts of it are based on the work below.

## Whale models (not MIT)

The whale models used by the Rust scene live in `assets/whales/prepared/` (the copy under
`macos/EchomereScene/Sources/CoveSceneRenderer/Resources/Whales/` remains until the Swift package is removed).
They are adapted from
"Model 55A - Southern Right Whale" by DigitalLife3D and "Female Orca" by neurodolphin, both under
CC BY-NC 4.0. See the `LICENSE.md` in that folder for sources, credits and the changes made.

## web-ocean, Maksymilian Dendura

The sea simulation in `macos/EchomereScene/Sources/CoveSceneRenderer/OceanShaders.swift` (the Tessendorf FFT
ocean: JONSWAP wind sea with Donelan-Banner spreading, swell, choppy displacement, frequencies quantised to a
time loop, and the radix-2 inverse FFT) is ported from web-ocean's `js/wgsl/sim.js`. Changes: translated from
WGSL to Metal, three cascades instead of six, no foam, and an extra pass for the surface curvature.
`scene/cove-render/shaders/ocean.wgsl` carries the same port back to WGSL for the Rust renderer, with the
same changes plus a pass that builds the curvature's mipmaps.

- Source: https://github.com/Noniv/web-ocean
- Copyright (c) 2026 Maksymilian Dendura
- Licence: MIT (text below)

## WebGL Water, Evan Wallace

The water optics in `macos/EchomereScene/Sources/CoveSceneRenderer/SceneShaders.swift` and its Rust port
under `scene/cove-render/shaders/` (refraction through
the wave normal, Snell's window, total internal reflection, and caustics from the curvature of the
refracting surface) follow the approach of WebGL Water. The code was re-derived for Echomere's camera and
wave model, not copied.

- Source: https://github.com/evanw/webgl-water
- Copyright 2011 Evan Wallace
- Licence: MIT (text below)

## MIT licence text (applies to both projects above)

> Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
> associated documentation files (the "Software"), to deal in the Software without restriction, including
> without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the
> following conditions:
>
> The above copyright notice and this permission notice shall be included in all copies or substantial
> portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT
> LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO
> EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
> IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
> USE OR OTHER DEALINGS IN THE SOFTWARE.

## abyssal-garden, Maksymilian Dendura

The finishing step in `scene/cove-render/shaders/composite.wgsl` (exposure, ACES tone map, luminance S-curve
and desaturation) is ported from abyssal-garden's final composite (`W_POST` in `src/shaders.js`), without its
light shafts and bloom. The water lighting model (extinction and diffuse attenuation per channel, in-scattered
light, ambient light) follows abyssal-garden's water as well.

- Source: https://github.com/noniv/abyssal-garden
- Copyright (c) 2026 Maksymilian Dendura
- Licence: MIT (text below, verbatim from the project's `LICENSE`)

```
MIT License

Copyright (c) 2026 Maksymilian Dendura

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Background reading (no code used)

- Jerry Tessendorf, "Simulating Ocean Water": the deep-water dispersion relation used for the waves.
