// Shared by every shader of the cove scene. Light is linear everywhere inside the scene; `finish()` in
// composite.wgsl turns it into display values once per output pixel.

// The water model's starting constants (lighting design), per metre. Starting values from abyssal-garden
// by Maksymilian Dendura (https://github.com/noniv/abyssal-garden, MIT); see THIRD-PARTY-NOTICES.md.
const SIGMA: vec3f = vec3f(0.058, 0.0235, 0.0195);       // extinction
const KD: vec3f = vec3f(0.056, 0.0245, 0.0170);          // diffuse attenuation of sunlight with depth
const WATER_COLOUR: vec3f = vec3f(0.16, 0.58, 0.90);
// Open water without a hit is drawn as if the ray met something this far away (m).
const OPEN_WATER: f32 = 240.0;
// Scales the linear light before the tone map. The design's starting value 0.6 washed the picture out
// with the water model (mean luma 0.37 on the moonlit test frame, 0.54 at 07:00, 0.63 at noon); 0.2 keeps
// it dark and calm; the owner chose 0.13 (2026-11-16) with the camera tilted 40 degrees up.
const EXPOSURE: f32 = 0.13;

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

// MARK: The water model
//
// One water model lights and fogs every element of the scene (lighting design,
// docs/superpowers/specs/2026-11-15-scene-lighting-design.md). All results are linear light.

// The camera stands on the seabed, 30 m down. World y is up, 0 at the mean sea surface.
const CAMERA_DEPTH: f32 = 30.0;
const CAMERA: vec3f = vec3f(0.0, -CAMERA_DEPTH, 0.0);
const WATER_IOR: f32 = 1.333;

// The sky at this moment (cove_scene::sky::SkyState): the sun by day, the moon by night. The colours are
// display-encoded, as `Sky.swift` picked them; they are decoded with `to_linear` where they are used.
struct Sky {
    light: vec3f,                  // direction towards the sun or the moon, above the water
    intensity: f32,
    colour: vec3f,
    ambient: f32,                  // how bright the water itself is
    zenith: vec3f,
    horizon: vec3f,
    night: f32,                    // 1 when the light is the moon's, 0 for the sun's
};

fn sky_from(light: vec4f, colour: vec4f, zenith: vec4f, horizon: vec4f) -> Sky {
    var s: Sky;
    s.light = normalize(light.xyz);
    s.intensity = light.w;
    s.colour = colour.rgb;
    s.ambient = colour.a;
    s.night = zenith.w;
    s.zenith = zenith.rgb;
    s.horizon = horizon.rgb;
    return s;
}

// Direction towards the light under water (the light bends towards the vertical as it enters).
fn sun_in_water(light: vec3f) -> vec3f {
    return -refract(-light, vec3f(0.0, 1.0, 0.0), 1.0 / WATER_IOR);
}

// The sun's or the moon's light at `depth` metres, per channel: red is lost first.
fn sunlight_at(depth: f32, s: Sky) -> vec3f {
    return to_linear(s.colour) * s.intensity * exp(-KD * depth / max(sun_in_water(s.light).y, 0.2));
}

// Adapted from abyssal-garden, Copyright (c) 2026 Maksymilian Dendura, MIT License.
// Light from the water around a surface facing `normal` at `depth` metres: brighter from above (`amb`,
// shaders.js:259-262, with sky = 1 and z = 0) plus a sideways fill (`side`, shaders.js:264, open = 1), all
// scaled by the sky's ambient.
fn ambient_light(depth: f32, normal: vec3f, s: Sky) -> vec3f {
    let up = normal.y * 0.5 + 0.5;
    let above = vec3f(0.22, 0.60, 0.80) * exp(-KD * depth * 1.2) * (0.24 + 0.76 * up * up) * 1.3
              * (0.55 + 0.45 * clamp(depth / 30.0, 0.0, 1.0)) + vec3f(0.003, 0.010, 0.016);
    let side = WATER_COLOUR * exp(-KD * depth) * (1.0 - abs(normal.y)) * 0.7;
    return (above + side) * s.ambient;
}

// Adapted from abyssal-garden, Copyright (c) 2026 Maksymilian Dendura, MIT License.
// Light scattered towards the camera along a line of sight from `origin` in direction `ray`, over
// `distance` metres: the depth-dependent attenuation integrated in closed form (`fogIn`, shaders.js:210-217).
// The diffused light is scaled by the sky's ambient; a forward-scattering lobe towards the sun or the moon
// by its intensity.
fn in_scatter(origin: vec3f, ray: vec3f, distance: f32, s: Sky) -> vec3f {
    var a = SIGMA - KD * ray.y;
    a = select(a, vec3(1e-4), abs(a) < vec3(1e-4));          // no division by ~0 for steep upward rays
    let integral = (1.0 - exp(-a * distance)) / a;
    let mu = max(dot(ray, sun_in_water(s.light)), 0.0);
    // Under water the diffused light comes mostly from above: looking sideways sees far less of it than
    // looking up, and looking down almost none.
    let up = ray.y * 0.5 + 0.5;
    let light = WATER_COLOUR * s.ambient * 1.22 * mix(0.02, 1.0, up * up * up * up)     // diffused light: never 0
              + WATER_COLOUR * to_linear(s.colour) * s.intensity * 0.3 * pow(mu, 4.0);  // glow towards sun/moon
    return SIGMA * exp(KD * origin.y) * integral * light;
}

// What reaches the camera from something of colour `c` at `distance` metres along `ray`.
fn through_water(c: vec3f, origin: vec3f, ray: vec3f, distance: f32, s: Sky) -> vec3f {
    return c * exp(-SIGMA * distance) + in_scatter(origin, ray, distance, s);
}

// The camera: a real perspective camera with a 70 degree vertical field of view, tilted 15 degrees up.
const FOCAL: f32 = 0.714;
const PITCH: f32 = 0.69813;               // 40 degrees up
const HORIZON: f32 = -0.0991;           // 0.5 - FOCAL * tan(PITCH): below the picture
// sea::CASCADE_SIZES: long swell, wind sea, ripples.
fn cascade_size(c: i32) -> f32 { return select(select(211.3, 39.7, c == 1), 7.43, c == 2); }

// Where a point in the water (metres) lands on the picture, as a clip-space position.
fn project(world: vec3f, aspect: f32) -> vec4f {
    let d = world - CAMERA;
    let cp = cos(PITCH);
    let sp = sin(PITCH);
    let yc = d.y * cp - d.z * sp;
    let zc = d.y * sp + d.z * cp;
    let near = 0.3;
    let far = 400.0;
    return vec4f(2.0 * FOCAL * d.x / aspect, 2.0 * FOCAL * yc, far / (far - near) * (zc - near), zc);
}

// The line of sight through the point `uv` (0..1 across the picture, y up).
fn view_ray(uv: vec2f, aspect: f32) -> vec3f {
    let u = (uv.x - 0.5) * aspect;
    let v = uv.y - 0.5;
    let cp = cos(PITCH);
    let sp = sin(PITCH);
    return normalize(vec3f(u, v * cp + FOCAL * sp, FOCAL * cp - v * sp));
}
