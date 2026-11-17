// The kelp frame: long ribbon blades rising at the picture's left and right edges. One mesh (a ribbon strip,
// src/kelp.rs) is drawn once per blade; this shader bends, twists and sways it. The fragment is lit by the
// water model of common.wgsl and writes linear light (`finish()` runs in the composite pass).
//
// The blade (`kelp_vertex`: its curl, the wave running down it, its twist, its ruffled edges, `sway` and
// `shape2`) is adapted from abyssal-garden's `W_LEAF` (`src/shaders.js:382-440`), Copyright (c) 2026
// Maksymilian Dendura, MIT License; see THIRD-PARTY-NOTICES.md. Changes: one kelp type (type 0's colours
// and shape, a gentler wave and curl for blades this long), no distance fades, no shadow pass, Echomere's own placement (blades lean so they keep their
// picture x, turned to the screen's shape), a gentle push from the swell, and time from the slowed kelp
// clock.

const ROWS: f32 = 48.0;                                   // BLADE_ROWS in kelp.rs
const REFERENCE_ASPECT: f32 = 1.6;                        // the picture shape the blades are placed for
const SWELL_PUSH: f32 = 0.3;                              // metres the swell moves a blade's tip sideways
const KELP_ALBEDO: f32 = 0.6;                             // lifts type 0's colours to the blade's albedo

// u[0]: aspect, swell surge (radians), kelp time (s), -
// u[1]: 1 / pixels high, warmth, -, -
// u[2]: light direction + intensity
// u[3]: light colour + ambient
@group(0) @binding(0) var<uniform> u: array<vec4f, 4>;

struct VIn {
    @location(0) mesh: vec2f,                             // across (-1 or 1), row (0..ROWS-1)
    @location(1) a: vec4f,                                // root x, y, z, length
    @location(2) b: vec4f,                                // lean, forward, width, curvature
    @location(3) c: vec4f,                                // seed 0, seed 1, side, sway phase
};

struct VOut {
    @builtin(position) position: vec4f,
    @location(0) world: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,                               // along (0 root, 1 tip) and across (-1..1) the blade
    @location(3) @interpolate(flat) colour: vec3f,
    @location(4) lift: f32,                               // metres along the blade from the root
};

fn sway(p: vec2f, h: f32, t: f32) -> vec2f {
    let ph = dot(p, vec2f(0.031, 0.017));
    let a = 0.024 * pow(max(h, 0.0), 1.25);
    return a * (vec2f(sin(t * 0.5 + ph + h * 0.09), cos(t * 0.37 + ph * 1.3 + h * 0.07))
              + 0.35 * vec2f(sin(t * 1.3 + ph * 3.0 + h * 0.3), cos(t * 1.1 + ph * 2.0 + h * 0.23)));
}

fn shape2(v: f32) -> f32 { return sin(3.14159 * saturate(v)); }

@vertex
fn kelp_vertex(in: VIn) -> VOut {
    let aspect = u[0].x;
    let t = u[0].z;
    let across = in.mesh.x;
    let v = sin(in.mesh.y / (ROWS - 1.0) * 1.5708);
    let len = in.a.w;
    let width = in.b.z;
    let s1 = in.c.x;
    let s2 = in.c.y;

    // Placed for 16:10; on other screens the root and the lean turn with the picture's width.
    let fit = aspect / REFERENCE_ASPECT;
    let root = vec3f(in.a.x * fit, in.a.y, in.a.z);
    let lean = in.b.x * fit;
    let forward = in.b.y;
    // Along the blade's line the sideways offset grows with the distance along the view axis, so the
    // whole blade keeps its picture x.
    let d = normalize(vec3f(lean * (sin(PITCH) + cos(PITCH) * forward), 1.0, forward));
    // Bending: perpendicular to the blade, in its vertical plane, away from up (the blade droops outwards).
    let up = vec3f(0.0, 1.0, 0.0);
    let pe = normalize(d * dot(up, d) - up);
    let sd = normalize(cross(d, pe));

    // Type 0 of W_LEAF (fls 1, shape (0.5, 0.32)), with a gentler wave (wav 0.6, not 1.9) and an
    // outward-only curl (0.10..0.35, not -0.6..1.4): these blades are 8 to 20 m long, not 1 to 3 m, and
    // W_LEAF's amounts would swing them metres into the middle of the picture or out of it.
    let wav = 0.6;
    let k = 0.10 + 0.25 * in.b.w + sin(t * (0.8 + s1) + s2 * 40.0) * 0.03;
    let wph = v * (3.0 + 5.0 * s2) + s1 * 30.0 + t * 1.1;
    let wv = sin(wph) * 0.12 * wav;
    let kq = k * 1.7;
    let ka = select(kq, 0.06, abs(kq) < 0.06);
    let sK = sin(ka * v);
    let cK = cos(ka * v);
    let c = root + len * (d * (sK / ka) + pe * ((1.0 - cK) / ka + wv * v));
    let sl = sK + wv + cos(wph) * 0.10 * wav * v * (3.0 + 5.0 * s2);
    let N = normalize(pe * cK - d * sl);
    let T0 = normalize(d * cK + pe * sl);
    let tw = (s1 - 0.5) * 2.4 * v + sin(t * 0.9 + s2 * 20.0) * 0.25 * v;
    let S = sd * cos(tw) + N * sin(tw);
    let N2 = N * cos(tw) - sd * sin(tw);
    let shape = pow(max(sin(3.14159 * pow(v, 0.5)), 0.0), 0.32);
    let ruf = sin(v * 10.0 + s2 * 30.0 + t * 0.6) * width * 0.36 * across * shape2(v);
    var p = c + S * (across * width * 0.5 * shape);
    p += N2 * ruf;
    let sw = sway(root.xz, p.y - root.y, t);
    p.x += sw.x;
    p.z += sw.y;
    // The swell pushes the whole frame gently back and forth, most at the tips.
    p.x += SWELL_PUSH * pow(v, 1.5) * sin(u[0].y + in.c.w);

    var o: VOut;
    o.world = p;
    o.normal = normalize(N2 + S * across * 0.55 - T0 * cos(v * 10.0 + s2 * 30.0 + t * 0.6) * across * 0.7);
    o.uv = vec2f(v, across);
    var colour = mix(vec3f(0.11, 0.125, 0.022), vec3f(0.04, 0.10, 0.04), s1);
    colour = mix(colour, vec3f(0.25, 0.17, 0.025), step(0.72, s2));
    o.colour = colour;
    o.lift = v * len;
    o.position = project(p, aspect);
    return o;
}

@fragment
fn kelp_fragment(in: VOut) -> @location(0) vec4f {
    // Only the light, its colour and the warmth are used; the sky's zenith and horizon are not needed here.
    let s = sky_from(u[2], u[3], vec4f(0.0), vec4f(0.0, 0.0, 0.0, u[1].y));
    let to_camera = CAMERA - in.world;
    let distance = length(to_camera);
    let view = to_camera / distance;
    var normal = normalize(in.normal);
    if (dot(normal, view) < 0.0) { normal = -normal; }
    let sun = sun_in_water(s.light);
    let depth = max(-in.world.y, 0.0);
    let sunlight = sunlight_at(depth, s);                 // per channel, includes the light's colour
    let ambient = ambient_light(depth, normal, s);

    let lit = saturate(dot(normal, sun));
    // Sunlight shining through a blade towards the camera glows amber, most at the thin edges.
    let behind = saturate(dot(-normal, sun) + 0.25);
    let towards = pow(saturate(dot(-view, sun) * 0.5 + 0.5), 2.0);
    let through = behind * towards * (0.65 + 0.35 * abs(in.uv.y));
    var c = KELP_ALBEDO * in.colour * (ambient + 0.5 * sunlight + 0.7 * lit * sunlight)
          + to_linear(vec3f(0.80, 0.62, 0.30)) * sunlight * through * 1.6;
    c *= 1.0 - 0.30 * smoothstep_f(0.18, 0.0, abs(in.uv.y));         // the midrib
    c *= 0.92 + 0.08 * sin(in.uv.x * 38.0 + in.colour.r * 50.0);     // the wrinkled surface

    // The water between the kelp and the camera; the root is lost in the gloom below.
    let seen = through_water(c, CAMERA, -view, distance, s);
    let gloom = in_scatter(CAMERA, -view, distance, s);
    return vec4f(mix(gloom, seen, smoothstep_f(0.0, 3.0, in.lift)), 1.0);
}
