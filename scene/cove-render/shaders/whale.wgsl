// A whale model (see tools/prepare-whales.py), swum by its own skeleton animation: every vertex follows up
// to four joints. The joint matrices put it in the whale's own space, where x runs from the flukes (-0.5)
// to the snout (+0.5) and y is up. Port of `whale_vertex` and `whale_fragment` from SceneShaders.swift,
// lit by the water model of common.wgsl; it writes linear light (`finish()` runs in the composite pass).

const MAX_JOINTS: i32 = 80;                               // whales::MAX_JOINTS
const WHALE_HEADER: i32 = 6;
// The skins are dark; decoded they read as black at 30 m. This lifts their albedo so the shape shows.
const WHALE_ALBEDO_LIFT: f32 = 3.0;

// w[0]: position, length · w[1]: forward, roll · w[2]: warmth, opacity, -, - · w[3]: aspect, 1 / pixels high
// w[4]: light direction, intensity · w[5]: light colour, ambient · from w[6]: three rows per joint
@group(0) @binding(0) var<uniform> w: array<vec4f, 246>;  // WHALE_HEADER + 3 * MAX_JOINTS
@group(0) @binding(1) var skin: texture_2d<f32>;
@group(0) @binding(2) var curvature: texture_2d_array<f32>;
@group(0) @binding(3) var sea_sampler: sampler;           // repeat, linear, linear mips: skin and sea

struct WhaleIn {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
    @location(3) joints: vec4u,
    @location(4) weights: vec4f,
};

struct WhaleOut {
    @builtin(position) position: vec4f,
    @location(0) world: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
};

@vertex
fn whale_vertex(in: WhaleIn) -> WhaleOut {
    let bind = vec4f(in.position, 1.0);
    let bind_normal = vec4f(in.normal, 0.0);
    var p = vec3f(0.0);
    var n = vec3f(0.0);
    for (var i = 0; i < 4; i++) {
        let weight = in.weights[i];
        if (weight <= 0.0) { continue; }
        let row = WHALE_HEADER + min(i32(in.joints[i]), MAX_JOINTS - 1) * 3;
        p += weight * vec3f(dot(w[row], bind), dot(w[row + 1], bind), dot(w[row + 2], bind));
        n += weight * vec3f(dot(w[row], bind_normal), dot(w[row + 1], bind_normal), dot(w[row + 2], bind_normal));
    }
    n = normalize(n);

    let forward = normalize(w[1].xyz);
    let right = normalize(cross(forward, vec3f(0.0, 1.0, 0.0)));
    let up = cross(right, forward);
    let c = cos(w[1].w);
    let s = sin(w[1].w);
    let rolled_up = up * c + right * s;
    let rolled_right = right * c - up * s;

    var o: WhaleOut;
    o.world = w[0].xyz + (forward * p.x + rolled_up * p.y + rolled_right * p.z) * w[0].w;
    o.normal = forward * n.x + rolled_up * n.y + rolled_right * n.z;
    o.uv = in.uv;
    o.position = project(o.world, w[3].x);
    return o;
}

@fragment
fn whale_fragment(in: WhaleOut) -> @location(0) vec4f {
    // Only the light, its colour and the warmth are used; the sky's zenith and horizon are not needed here.
    let s = sky_from(w[4], w[5], vec4f(0.0), vec4f(0.0, 0.0, 0.0, w[2].x));
    let to_camera = CAMERA - in.world;
    let distance = length(to_camera);
    let view = to_camera / distance;
    var normal = normalize(in.normal);
    if (dot(normal, view) < 0.0) { normal = -normal; }
    let sun = sun_in_water(s.light);
    let depth = max(-in.world.y, 0.0);

    // Sunlight focused by the waves dapples the whale's back.
    let dapple = mix(1.0, caustic_at(s.light, in.world.xz, depth, false), smoothstep_f(0.0, 0.7, normal.y));

    let albedo = min(WHALE_ALBEDO_LIFT * to_linear(textureSample(skin, sea_sampler, in.uv).rgb), vec3f(0.9));
    let c = albedo * (ambient_light(depth, normal, s)
                      + sunlight_at(depth, s) * saturate(dot(normal, sun)) * 2.2 * dapple);

    // The water between the whale and the camera. A whale fades in when it arrives: blended by its opacity.
    return vec4f(through_water(c, CAMERA, -view, distance, s), w[2].y);
}
