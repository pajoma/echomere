// The composite pass: upscales the half-resolution scene to the output with a bilinear filter and
// finishes every output pixel exactly once.
//
// `finish` (tone map, S-curve and desaturation) is ported from abyssal-garden's `W_POST` by Maksymilian
// Dendura (https://github.com/noniv/abyssal-garden, `src/shaders.js`), MIT licence, Copyright (c) 2026
// Maksymilian Dendura; see THIRD-PARTY-NOTICES.md. Changes: no light shafts and no bloom, the vignette
// and dither are Echomere's own, and the vignette comes after the tone map.

@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var bilinear: sampler;

struct VOut {
    @builtin(position) position: vec4f,
    // 0..1 across the output, y down (texture space).
    @location(0) uv: vec2f,
};

@vertex
fn composite_vertex(@builtin(vertex_index) vid: u32) -> VOut {
    let p = vec2f(f32((vid << 1u) & 2u), f32(vid & 2u));
    var o: VOut;
    o.position = vec4f(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2f(p.x, 1.0 - p.y);
    return o;
}

// ACES filmic tone map (Narkowicz fit).
fn tmap(x: vec3f) -> vec3f {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3f(0.0), vec3f(1.0));
}

// Linear scene light to display values: exposure, ACES, a gentle luminance S-curve, a little
// desaturation, the vignette, gamma encode and dither. `uv` is 0..1 across the output, `pixel` the
// output pixel's position.
fn finish(colour: vec3f, uv: vec2f, pixel: vec2f) -> vec3f {
    var c = colour * EXPOSURE;
    c = tmap(c);
    let l0 = dot(c, vec3f(0.2126, 0.7152, 0.0722));
    let l = mix(l0, l0 * l0 * (3.0 - 2.0 * l0), 0.55);
    c *= l / max(l0, 1e-4);
    c = mix(c, vec3f(l), smoothstep_f(0.55, 1.0, l) * 0.1);
    c = mix(vec3f(l), c, 0.93) + vec3f(0.008, 0.002, 0.0);
    let v = (uv - 0.5) * vec2f(1.0, 0.8);
    c *= 1.0 - 0.35 * dot(v, v);
    c = pow(max(c, vec3f(0.0)), vec3f(1.0 / 2.2));
    return c + (hash21(pixel) - 0.5) / 255.0;
}

@fragment
fn composite_fragment(in: VOut) -> @location(0) vec4f {
    let c = textureSampleLevel(scene, bilinear, in.uv, 0.0).rgb;
    return vec4f(clamp(finish(c, in.uv, in.position.xy), vec3f(0.0), vec3f(1.0)), 1.0);
}
