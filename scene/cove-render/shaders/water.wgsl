// The water pass: one full-screen triangle that draws the open water, the underside of the surface,
// echoes, marine snow and the light shafts, in linear light into the scene target. Port of `scene_fragment`
// and its helpers in `SceneShaders.swift`, with the lighting design's water model (common.wgsl). There is
// no `finish()` here: the composite pass does it.
//
// The optics (refraction through the wave normal, Snell's window, total internal reflection, and caustics
// from the curvature of the refracting surface) follow the approach of Evan Wallace's WebGL Water
// (https://github.com/evanw/webgl-water, MIT licence, Copyright 2011 Evan Wallace), re-derived for this
// camera; see THIRD-PARTY-NOTICES.md.
//
// The surface's reflected water (`in_scatter` along the mirrored ray, times 1.15) is adapted from
// abyssal-garden, Copyright (c) 2026 Maksymilian Dendura, MIT License (`shaders.js:604-605`).

// Must match `SceneBuffer` in water.rs.
const SKY: i32 = 2;
const CREATURES: i32 = 14;        // the small fish (drawn in their own pass, not here)
const ECHOES: i32 = 326;
const COUNT: i32 = 336;

// The camera: a real perspective camera with a 70 degree vertical field of view, tilted 15 degrees up.
const FOCAL: f32 = 0.714;
const PITCH: f32 = 0.262;
const HORIZON: f32 = 0.309;            // 0.5 - FOCAL * tan(PITCH)
// sea::CASCADE_SIZES: long swell, wind sea, ripples.
fn cascade_size(c: i32) -> f32 { return select(select(211.3, 39.7, c == 1), 7.43, c == 2); }
const MAX_WAVE: f32 = 3.0;             // no wave rises or dips further than this, in metres

// Distances of the 2D elements (lighting design), in metres.
const ECHO_DISTANCE: f32 = 20.0;
const SNOW_NEAR: f32 = 2.0;             // layer 0
const SNOW_FAR: f32 = 6.0;              // layer 1
// Overall strength of the light shafts.
const SHAFTS: f32 = 1.0;

// d[0]: aspect, shader time (s within the day), creature count, -
// d[1]: echo count, 1 / pixels high, sea from above (0 or 1), -
// d[SKY..SKY+3]: light direction + intensity, light colour + ambient, zenith, horizon
// d[CREATURES + 3i ..]: x, y, depth, facing | phase, size, 1, - | colour, 1, -, -
// d[ECHOES + 2i ..]: x, y, depth, intensity | call (or -1), size, -, -
@group(0) @binding(0) var<uniform> d: array<vec4f, COUNT>;
@group(0) @binding(1) var displacement: texture_2d_array<f32>;   // per cascade: Dx, height, Dz, dDx/dz
@group(0) @binding(2) var derivatives: texture_2d_array<f32>;    // per cascade: slope x, slope z, dDx/dx, dDz/dz
@group(0) @binding(3) var curvature: texture_2d_array<f32>;      // per cascade: d2h/dx2, d2h/dxdz, d2h/dz2, -
@group(0) @binding(4) var sea_sampler: sampler;                  // repeat, linear, linear mips

struct VOut {
    @builtin(position) position: vec4f,
    // 0..1 across the scene target, y up.
    @location(0) uv: vec2f,
};

@vertex
fn water_vertex(@builtin(vertex_index) vid: u32) -> VOut {
    let p = vec2f(f32((vid << 1u) & 2u), f32(vid & 2u));
    var o: VOut;
    o.position = vec4f(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = p;
    return o;
}

fn view_ray(uv: vec2f, aspect: f32) -> vec3f {
    let u = (uv.x - 0.5) * aspect;
    let v = uv.y - 0.5;
    let cp = cos(PITCH);
    let sp = sin(PITCH);
    return normalize(vec3f(u, v * cp + FOCAL * sp, FOCAL * cp - v * sp));
}

// The same sea seen from 4 m above, looking towards the sun: for checking the waves.
fn view_ray_above(uv: vec2f, aspect: f32) -> vec3f {
    let u = (uv.x - 0.5) * aspect;
    let v = uv.y - 0.5;
    let cp = cos(-0.14);
    let sp = sin(-0.14);
    return normalize(vec3f(u, v * cp + FOCAL * sp, FOCAL * cp - v * sp));
}

fn sea_sample(t: texture_2d_array<f32>, q: vec2f, cascade: i32, level: f32) -> vec4f {
    return textureSampleLevel(t, sea_sampler, q / cascade_size(cascade), cascade, level);
}

// The slope of the surface at q (metres), summed over the cascades. Ripples finer than a pixel are faded
// out with distance; the choppy crests (horizontal displacement) steepen the slope.
fn slope_at(q: vec2f, distance: f32) -> vec2f {
    var slope = vec2f(0.0);
    var jxx = 0.0;
    var jzz = 0.0;
    for (var c = 0; c < 3; c++) {
        let fade = select(select(1.0, exp(-distance / 200.0), c == 1), exp(-distance / 30.0), c == 2);
        let s = sea_sample(derivatives, q, c, 0.0);
        slope += s.xy * fade;
        jxx += s.z * fade;
        jzz += s.w * fade;
    }
    return slope / max(vec2f(1.0 + jxx, 1.0 + jzz), vec2f(0.25));
}

// Curvature of the surface. `broad` keeps only the long waves: seen along a line of sight through the
// water, the fine caustic net averages out and only the broad focusing shows as light shafts.
fn curvature_at(q: vec2f, broad: bool) -> vec4f {
    let c = sea_sample(curvature, q, 0, 0.0);
    return c + sea_sample(curvature, q, 1, select(0.0, 2.5, broad));
}

// How strongly the waves focus sunlight at `depth` metres below surface point `q`: the inverse of how much
// a small bundle of refracted sunlight has spread (< 1) or narrowed (> 1) on its way down.
fn focus_at(q: vec2f, depth: f32, broad: bool) -> f32 {
    let c = curvature_at(q, broad);
    // The sun is a disk, not a point, so deep caustics blur: the focusing grows more slowly with depth.
    // The broad version already reads blurred curvature, so it needs less of this.
    let blurred = depth / (1.0 + depth / select(8.0, 20.0, broad));
    let k = blurred * (1.0 - 1.0 / WATER_IOR);
    let jacobian = (1.0 + k * c.x) * (1.0 + k * c.z) - (k * c.y) * (k * c.y);
    return min(1.0 / max(abs(jacobian), 0.15), 4.0);
}

// Caustics on something at `depth` metres: the focused light, following the sun's slant to the surface.
fn caustic_at(light: vec3f, position: vec2f, depth: f32, broad: bool) -> f32 {
    let sun = sun_in_water(light);
    let q = position + sun.xz * (depth / sun.y);
    return focus_at(q, depth, broad);
}

// Height of the long waves at the point x (metres). The choppy crests are waves pushed sideways, so the
// sideways push is undone first to find which part of the wave lies above x.
fn height_at(x: vec2f) -> f32 {
    let push = sea_sample(displacement, x, 0, 0.0);
    return sea_sample(displacement, x - push.xz, 0, 0.0).y;
}

// Where a line of sight meets the waves: the distance along the ray, or a negative value if it never does.
// Works from below and from above. The ray is walked through the layer the waves occupy until it crosses
// the surface, then the crossing is refined.
fn surface_hit(origin: vec3f, ray: vec3f) -> f32 {
    if (abs(ray.y) < 1e-4) { return -1.0; }
    let side = sign(ray.y);
    let start = max((-MAX_WAVE * side - origin.y) / ray.y, 0.0);
    let end = min((MAX_WAVE * side - origin.y) / ray.y, 600.0);
    if (end <= start) { return -1.0; }
    let steps = 10;
    let dt = (end - start) / f32(steps);
    var before = start;
    var gap_before = side * (origin.y + start * ray.y - height_at(origin.xz + ray.xz * start));
    for (var i = 1; i <= steps; i++) {
        var t = start + dt * f32(i);
        var gap = side * (origin.y + t * ray.y - height_at(origin.xz + ray.xz * t));
        if (gap > 0.0) {
            for (var k = 0; k < 2; k++) {                    // refine between the last two steps
                let middle = before + (t - before) * (-gap_before / (gap - gap_before));
                let gap_middle = side * (origin.y + middle * ray.y - height_at(origin.xz + ray.xz * middle));
                if (gap_middle > 0.0) {
                    t = middle;
                    gap = gap_middle;
                } else {
                    before = middle;
                    gap_before = gap_middle;
                }
            }
            return before + (t - before) * (-gap_before / (gap - gap_before));
        }
        before = t;
        gap_before = gap;
    }
    return end;
}

// The upward normal of the surface above the point x, from all three wave scales.
fn normal_at(x: vec2f, distance: f32) -> vec3f {
    let push = sea_sample(displacement, x, 0, 0.0).xz + sea_sample(displacement, x, 1, 0.0).xz;
    let slope = slope_at(x - push, distance);
    return normalize(vec3f(-slope.x, 1.0, -slope.y));
}

// The sky and the sun, as seen in direction `direction` above the water, in linear light. The sky's
// display-encoded colours are decoded before the sun's disc and glow are added on top.
fn sky_colour(s: Sky, direction: vec3f) -> vec3f {
    let towards = clamp(dot(direction, s.light), 0.0, 1.0);
    var c = mix(to_linear(s.horizon), to_linear(s.zenith), pow(clamp(direction.y, 0.0, 1.0), 0.7));
    // The disc of the sun or the moon, and the glow around it.
    c += to_linear(s.colour) * s.intensity
       * (pow(towards, 3000.0) * 60.0 + pow(towards, 200.0) * 3.0 + pow(towards, 10.0) * 0.7);
    return c;
}

// The underside of the surface, drawn where the line of sight really meets the waves; open water where it
// does not. Inside Snell's window the sky and the sun shine through; outside it the surface is a mirror
// for the water below. Every wave tilts the surface, so the edge of the window, the sun's glitter and the
// mirrored water all move with the waves.
fn surface_underside(s: Sky, ray: vec3f) -> vec3f {
    let open = in_scatter(CAMERA, ray, OPEN_WATER, s);
    if (ray.y <= 0.0) { return open; }
    let t = surface_hit(CAMERA, ray);
    if (t < 0.0) { return open; }
    let hit = CAMERA + ray * t;
    let normal = normal_at(hit.xz, t);

    // The mirror shows the water below, lit by the same water model.
    let mirrored = reflect(ray, -normal);
    let reflected = in_scatter(hit, mirrored, OPEN_WATER, s) * 1.15;
    let through = refract(ray, -normal, WATER_IOR);
    var c: vec3f;
    if (dot(through, through) < 1e-5) {
        c = reflected;                                       // total internal reflection
    } else {
        let fresnel = 0.02 + 0.98 * pow(1.0 - clamp(dot(through, normal), 0.0, 1.0), 5.0);
        c = mix(sky_colour(s, through), reflected, fresnel);
    }
    // Light reaching the camera through the water turns blue and fades with distance.
    return through_water(c, CAMERA, ray, t, s);
}

// The sea from above, for checking the waves against footage of a real sea.
fn sea_from_above(s: Sky, ray: vec3f) -> vec3f {
    let origin = vec3f(0.0, 4.0, 0.0);
    let horizon = sky_colour(s, normalize(vec3f(ray.x, max(ray.y, 0.0) + 0.02, ray.z)));
    if (ray.y >= 0.0) { return horizon; }
    let t = surface_hit(origin, ray);
    if (t < 0.0) { return horizon; }
    let hit = origin + ray * t;
    let normal = normal_at(hit.xz, t);
    var mirrored = reflect(ray, normal);
    mirrored.y = abs(mirrored.y);
    let fresnel = 0.02 + 0.98 * pow(1.0 - clamp(dot(-ray, normal), 0.0, 1.0), 5.0);
    let c = mix(to_linear(vec3f(0.010, 0.060, 0.090)) * s.ambient, sky_colour(s, mirrored), fresnel);
    return mix(horizon, c, exp(-t / 900.0));
}

// Light shafts: sunlight scattered towards the camera along its line of sight. At every step the light is
// as bright as the long waves above have focused it, so the shafts sway and flicker with the waves; a shaft
// is a line through the water along the sunlight, so collecting along the line of sight draws it as a
// streak. Seawater scatters mostly forwards, so the shafts are strongest when looking towards the sun. The
// light at each step is the water model's sunlight at that depth, scattered by the water and dimmed on its
// way to the camera.
fn light_shafts(s: Sky, ray: vec3f, pixel: vec2f, y: f32) -> vec3f {
    let sun = sun_in_water(s.light);
    let g = 0.55;
    let mu = dot(ray, sun);
    let phase = (1.0 - g * g) / pow(1.0 + g * g - 2.0 * g * mu, 1.5);
    let steps = 24;
    let range = select(32.0, min(32.0, (CAMERA_DEPTH - 0.3) / ray.y), ray.y > 0.0);
    let dt = range / f32(steps);
    // A fixed per-pixel offset hides the steps without flickering from frame to frame.
    let offset = hash21(floor(pixel));
    var light = vec3f(0.0);
    for (var i = 0; i < steps; i++) {
        let t = (f32(i) + offset) * dt;
        let point = CAMERA + ray * t;
        let depth = -point.y;
        if (depth < 0.3) { continue; }
        let focus = caustic_at(s.light, point.xz, depth, true);
        light += ((focus - 1.0) * 1.8 + 0.10) * sunlight_at(depth, s) * exp(-SIGMA * t) * dt;
    }
    // Only very few rays reach the deep in the lower third of the picture.
    let reach = mix(0.12, 1.0, smoothstep_f(0.08, HORIZON + 0.22, y));
    return SHAFTS * max(light, vec3f(0.0)) * SIGMA * WATER_COLOUR * min(phase, 6.0) * reach;
}

// An echo: the shimmer a whale left behind, and the ring of its call. Emitted light, `ECHO_DISTANCE` away.
fn draw_echo(p: vec2f, a: vec4f, b: vec4f, t: f32, i: i32) -> vec3f {
    let r = 0.06 * b.y * (1.15 - 0.6 * a.z);
    let rel = p - a.xy;
    let dd = length(rel / vec2f(1.8, 1.0));
    let shimmer = a.w * exp(-dd * dd / (r * r))
                * (0.55 + 0.45 * sin(t * 0.6 + f32(i) * 2.1 + p.x * 40.0 + p.y * 25.0));
    var c = to_linear(vec3f(0.10, 0.20, 0.24)) * shimmer * 0.6;
    if (b.x >= 0.0) {
        let radius = b.x * 0.28;
        let off_ring = (length(rel) - radius) / 0.006;
        let ring = exp(-off_ring * off_ring);
        c += to_linear(vec3f(0.12, 0.22, 0.26)) * ring * (1.0 - b.x) * a.w * 0.5;
    }
    return c * exp(-SIGMA * ECHO_DISTANCE);
}

// Marine snow, drifting down, in two layers (`SNOW_NEAR` and `SNOW_FAR` away). Emitted light.
fn marine_snow(p: vec2f, t: f32, s: Sky) -> vec3f {
    var col = vec3f(0.0);
    for (var l = 0; l < 2; l++) {
        let fl = f32(l);
        let sc = select(32.0, 60.0, l == 0);
        let g = p * sc + vec2f(sin(t * 0.05 + fl) * 2.0, t * (0.012 + 0.01 * fl) * sc);
        let cell = floor(g);
        let f = fract(g) - 0.5;
        let h = hash21(cell + fl * 17.0);
        let off = vec2f(hash21(cell + 3.1), hash21(cell + 7.7)) - 0.5;
        let m = smoothstep_f(0.07 + 0.04 * fl, 0.0, length(f - off * 0.6)) * step(0.86, h);
        col += to_linear(vec3f(0.05, 0.07, 0.07)) * m * (0.5 + 0.5 * fl) * (0.3 + 0.7 * min(s.ambient, 1.0))
             * exp(-SIGMA * select(SNOW_FAR, SNOW_NEAR, l == 0));
    }
    return col;
}

@fragment
fn water_fragment(in: VOut) -> @location(0) vec4f {
    let aspect = d[0].x;
    let t = d[0].y;
    let echo_count = i32(d[1].x);
    let p = vec2f(0.8 + (in.uv.x - 0.5) * aspect, in.uv.y);

    let s = sky_from(d[SKY], d[SKY + 1], d[SKY + 2], d[SKY + 3]);
    if (d[1].z > 0.5) {
        return vec4f(sea_from_above(s, view_ray_above(in.uv, aspect)), 1.0);
    }
    let ray = view_ray(in.uv, aspect);
    var col = surface_underside(s, ray);
    for (var i = 0; i < echo_count; i++) {
        col += draw_echo(p, d[ECHOES + i * 2], d[ECHOES + i * 2 + 1], t, i);
    }
    col += marine_snow(p, t, s);
    col += light_shafts(s, ray, in.position.xy, in.uv.y);
    return vec4f(col, 1.0);
}
