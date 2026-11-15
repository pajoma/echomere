// Shared by every shader of the cove scene. Light is linear everywhere inside the scene; `finish()` in
// composite.wgsl turns it into display values once per output pixel.

// The water model's starting constants (lighting design), per metre. Starting values from abyssal-garden
// by Maksymilian Dendura (https://github.com/noniv/abyssal-garden, MIT); see THIRD-PARTY-NOTICES.md.
const SIGMA: vec3f = vec3f(0.058, 0.0235, 0.0195);       // extinction
const KD: vec3f = vec3f(0.056, 0.0245, 0.0170);          // diffuse attenuation of sunlight with depth
const WATER_COLOUR: vec3f = vec3f(0.16, 0.58, 0.90);
// Open water without a hit is drawn as if the ray met something this far away (m).
const OPEN_WATER: f32 = 240.0;
// Scales the linear light before the tone map.
const EXPOSURE: f32 = 0.6;

// Hand-picked, display-encoded colours enter the shaders through this.
fn to_linear(c: vec3f) -> vec3f { return pow(max(c, vec3f(0.0)), vec3f(2.2)); }

fn hash21(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn smoothstep_f(a: f32, b: f32, x: f32) -> f32 {
    let t = clamp((x - a) / (b - a), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}
