// The fish: one small 3D fish (src/fish.rs) drawn once per fish of the schools. The vertex shader bends the
// body with a swimming wave that grows towards the tail and turns the fish along its heading; the fragment
// is lit by the water model of common.wgsl, with a dark back, a light belly and, for the baitfish, the
// mirror-like glint of a silver flank turned towards the light. Linear light out; `finish()` runs in the
// composite pass.

// u[0]: aspect, -, -, -
// u[1]: 1 / pixels high, warmth, -, -
// u[2]: light direction + intensity
// u[3]: light colour + ambient
@group(0) @binding(0) var<uniform> u: array<vec4f, 4>;

// The swimming wave: how far the tail swings, in body lengths, and how many waves fit on the body.
const SWING: f32 = 0.11;
const WAVES: f32 = 1.1;
// How far a fish rolls from side to side, in radians.
const ROLL: f32 = 0.45;

struct VIn {
    @location(0) mesh: vec4f,                             // x (tail -0.5 .. nose 0.5), y, z, fin
    @location(1) a: vec4f,                                // position, length (m)
    @location(2) b: vec4f,                                // heading, speed (m/s)
    @location(3) c: vec4f,                                // kind (0 baitfish, 1 mackerel), tail phase, -, -
};

struct VOut {
    @builtin(position) position: vec4f,
    @location(0) world: vec3f,
    @location(1) normal: vec3f,
    @location(2) local: vec3f,                            // the mesh point, before bending
    @location(3) @interpolate(flat) kind: f32,
};

@vertex
fn fish_vertex(in: VIn) -> VOut {
    let aspect = u[0].x;
    let mackerel = in.c.x > 0.5;
    // Body height and width as fractions of the length: the baitfish slim, the mackerel deeper.
    let height = select(0.17, 0.21, mackerel);
    let width = select(0.10, 0.13, mackerel);
    let x = in.mesh.x;
    // The wave travels from head to tail; the swing grows towards the tail.
    let swing = SWING * pow(0.5 - min(x, 0.5), 2.0);
    let wave = WAVES * 6.2832 * x - in.c.y;
    var local = vec3f(x, in.mesh.y * height, in.mesh.z * width);
    local.z += swing * sin(wave);
    // The body's slope from the wave turns its normal.
    let slope = swing * WAVES * 6.2832 * cos(wave);

    let forward = normalize(in.b.xyz);
    var side = cross(vec3f(0.0, 1.0, 0.0), forward);
    side = select(normalize(side), vec3f(1.0, 0.0, 0.0), length(side) < 1e-4);
    let level_up = cross(forward, side);
    // Each fish rolls slowly a little from side to side, out of step with its neighbours. Seen from below,
    // that is what makes a silver flank catch the light now and then: the flicker of a baitfish school.
    let roll = ROLL * sin(in.c.y * 0.23 + in.a.x * 1.7 + in.a.z * 2.3);
    let up = level_up * cos(roll) + side * sin(roll);
    side = side * cos(roll) - level_up * sin(roll);
    let length = in.a.w;

    // The body's normal is the ellipse's around the axis; the fin's points sideways; the nose and tail tips
    // point along the axis. The wave's slope tilts it.
    let around = vec3f(0.0, in.mesh.y / height, in.mesh.z / width);
    var n = select(normalize(around), vec3f(sign(x), 0.0, 0.0), dot(around, around) < 1e-8);
    n = select(n, vec3f(0.0, 0.0, 1.0), in.mesh.w > 0.5);
    n = normalize(n - vec3f(slope * n.z, 0.0, 0.0));

    var o: VOut;
    o.world = in.a.xyz + (forward * local.x + up * local.y + side * local.z) * length;
    o.normal = forward * n.x + up * n.y + side * n.z;
    o.local = vec3f(x, in.mesh.y, in.mesh.z);
    o.kind = in.c.x;
    o.position = project(o.world, aspect);
    return o;
}

@fragment
fn fish_fragment(in: VOut) -> @location(0) vec4f {
    let s = sky_from(u[2], u[3], vec4f(0.0), vec4f(0.0, 0.0, 0.0, u[1].y));
    let to_camera = CAMERA - in.world;
    let distance = length(to_camera);
    let view = to_camera / distance;
    var normal = normalize(in.normal);
    if (dot(normal, view) < 0.0) { normal = -normal; }
    let sun = sun_in_water(s.light);
    let depth = max(-in.world.y, 0.0);
    let sunlight = sunlight_at(depth, s);
    let ambient = ambient_light(depth, normal, s);
    let mackerel = in.kind > 0.5;

    // Counter-shading: a dark back, a light belly (the mesh's y is up on the body).
    let belly = smoothstep_f(0.15, -0.25, in.local.y);
    var back = select(vec3f(0.035, 0.07, 0.10), vec3f(0.03, 0.07, 0.06), mackerel);
    if (mackerel) {
        // The mackerel's dark wavy bars on the back.
        back *= 0.6 + 0.4 * step(0.0, sin(in.local.x * 60.0 + in.local.z * 25.0));
    }
    let underside = select(vec3f(0.45, 0.50, 0.53), vec3f(0.35, 0.38, 0.36), mackerel);
    let albedo = mix(back, underside, belly);
    let lit = saturate(dot(normal, sun));
    var c = albedo * (ambient + 0.6 * lit * sunlight);
    if (!mackerel) {
        // A silver flank is a mirror: it flashes when it reflects the light towards the camera.
        let mirror = saturate(dot(reflect(-sun, normal), view));
        c += sunlight * pow(mirror, 12.0) * 2.5 * (1.0 - 0.6 * belly);
    }
    return vec4f(through_water(c, CAMERA, -view, distance, s), 1.0);
}
