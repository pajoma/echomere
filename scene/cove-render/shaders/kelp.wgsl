// Giant kelp as geometry. One mesh (src/kelp.rs) is drawn once per stalk; this shader bends the stalk
// through the water and hangs the blades on it. Port of `kelp_vertex` from SceneShaders.swift; the
// fragment is lit by the water model of common.wgsl and writes linear light (`finish()` runs in the
// composite pass).

const BLADES: f32 = 64.0;                                 // KELP_BLADES in kelp.rs
const CURRENT: vec2f = vec2f(0.80, 0.60);                 // the way the current flows (x, z)
const SURGE: vec2f = vec2f(0.60, 0.80);                   // the way the swell pushes the water back and forth

// u[0]: aspect, surge, drift, flutter
// u[1]: 1 / pixels high, -, -, -
// u[2]: light direction + intensity
// u[3]: light colour + ambient
@group(0) @binding(0) var<uniform> u: array<vec4f, 4>;
@group(0) @binding(1) var displacement: texture_2d_array<f32>;
@group(0) @binding(2) var sea_sampler: sampler;

struct VIn {
    @location(0) data: vec4f,                             // s, u, v, blade
    @location(1) a: vec4f,                                // base x, base depth, base z, length
    @location(2) b: vec4f,                                // sway phase, twist phase, blade seed, blade scale
};

struct VOut {
    @builtin(position) position: vec4f,
    @location(0) world: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,                               // along and across a blade
    @location(3) @interpolate(flat) blade: f32,           // < 0 for the stalk
    @location(4) lift: f32,                               // metres above the root
};

// Height of the long waves at the point x (metres), as in water.wgsl.
fn height_of(x: vec2f) -> f32 {
    let push = textureSampleLevel(displacement, sea_sampler, x / cascade_size(0), 0, 0.0);
    return textureSampleLevel(displacement, sea_sampler, (x - push.xz) / cascade_size(0), 0, 0.0).y;
}

// A point on the stalk (xyz) and how high it is (w): 0 at the root, 1 at the surface. s runs 0..1 from
// root to tip.
fn stalk_point(a: vec4f, b: vec4f, s: f32) -> vec4f {
    let depth = a.y;
    let along = s * a.w;
    let top = depth - 0.6;                                // the stalk levels off just below the surface
    let below = 1.5 * log(1.0 + exp((top - along) / 1.5));
    var y = -0.6 - below;
    let height = top - below;
    let trail = max(along - height, 0.0);                 // the part lying along the surface
    let rise = clamp(height / top, 0.0, 1.0);

    var xz = a.xz;
    xz += CURRENT * (trail * 0.9 + 0.20 * height * rise);               // leans and trails with the current
    xz += SURGE * (0.52 + 0.79 * rise) * pow(rise, 1.2) * sin(u[0].y - 1.0 * s + b.x);
    xz += vec2f(-CURRENT.y, CURRENT.x) * 0.3 * rise * sin(u[0].z + b.x * 1.7);
    // The stalks of one plant wind loosely around each other.
    xz += 0.16 * vec2f(cos(along * 0.45 + b.y), sin(along * 0.45 + b.y)) * smoothstep_f(0.0, 3.0, along);
    // The canopy rides the waves.
    y += smoothstep_f(3.0, 0.5, -y) * height_of(xz);
    return vec4f(xz.x, y, xz.y, rise);
}

@vertex
fn kelp_vertex(in: VIn) -> VOut {
    let a = in.a;
    let b = in.b;
    let aspect = u[0].x;
    var o: VOut;
    o.blade = in.data.w;
    var rise: f32;

    if (in.data.w < 0.0) {                                // the stalk: a ribbon that faces the camera
        let s = in.data.x;
        let here = stalk_point(a, b, s);
        let centre = here.xyz;
        let ahead = stalk_point(a, b, min(s + 0.01, 1.0)).xyz;
        let behind = stalk_point(a, b, max(s - 0.01, 0.0)).xyz;
        let tangent = normalize(ahead - behind);
        let to_camera = normalize(CAMERA - centre);
        let side = normalize(cross(tangent, to_camera));
        // Never thinner than about a pixel, so distant stalks don't break up.
        let width = max(0.065, length(CAMERA - centre) * u[1].x * 0.9);
        o.world = centre + side * in.data.z * width;
        o.normal = to_camera;
        o.uv = vec2f(s, in.data.z);
        rise = here.w;
    } else {
        let j = in.data.w;
        let h1 = hash21(vec2f(j, b.z));
        let h2 = hash21(vec2f(j + 17.0, b.z));
        let h3 = hash21(vec2f(j + 41.0, b.z));
        // Blades sit closer together towards the top.
        let s = 0.06 + 0.94 * pow((j + h1) / BLADES, 0.75);
        let root = stalk_point(a, b, s);
        let base = root.xyz;
        rise = root.w;

        let theta = j * 2.39996 + b.z;
        // Blades hang off the stalk in all directions and stream with the current.
        var direction = vec3f(cos(theta), 0.1 + 0.5 * (h2 - 0.5), sin(theta)) * 0.9
                      + vec3f(CURRENT.x, 0.0, CURRENT.y) * 0.45;
        // Blades trail behind the stalk as the surge moves it.
        let velocity = cos(u[0].y - 1.0 * s + b.x);
        direction -= vec3f(SURGE.x, 0.0, SURGE.y) * velocity * 0.6 * rise;
        direction = normalize(direction);
        let across = normalize(cross(direction, vec3f(0.0, 1.0, 0.0)));
        var normal = cross(across, direction);
        let twist = h3 * 6.2832;
        let turned = across * cos(twist) + normal * sin(twist);
        normal = cross(turned, direction);

        let along = in.data.y;
        let blade_length = (1.30 + 1.00 * h2) * b.w;
        let flutter = sin(u[0].w * select(2.0, 1.0, h1 < 0.5) + j * 1.3);
        let point = base + direction * blade_length * along
                  + normal * (0.12 * along * along * flutter + 0.05 * along * sin(along * 9.0 + j))
                  // Blades curl: most upwards towards the light, some downwards.
                  + vec3f(0.0, select(-0.35, 0.45, h3 < 0.75), 0.0) * along * along * blade_length
                  + turned * 0.25 * blade_length * along * along * (h1 - 0.5);
        let half_width = 0.150 * b.w * pow(sin(3.14159 * pow(along, 0.50)), 0.9)
                       + 0.022 * exp(-pow(along / 0.06, 2.0));          // the gas bladder at the base
        o.world = point + turned * in.data.z * half_width;
        o.normal = normal;
        o.uv = vec2f(along, in.data.z);
    }
    o.lift = rise * (a.y - 0.6);
    o.position = project(o.world, aspect);
    return o;
}

@fragment
fn kelp_fragment(in: VOut) -> @location(0) vec4f {
    // Only the light and its colour are used; the sky's zenith and horizon are not needed here.
    let s = sky_from(u[2], u[3], vec4f(0.0), vec4f(0.0));
    let to_camera = CAMERA - in.world;
    let distance = length(to_camera);
    let view = to_camera / distance;
    var normal = normalize(in.normal);
    if (dot(normal, view) < 0.0) { normal = -normal; }
    let sun = sun_in_water(s.light);
    let depth = max(-in.world.y, 0.0);
    let sunlight = sunlight_at(depth, s);                 // per channel, includes the light's colour
    let ambient = ambient_light(depth, normal, s);

    var c: vec3f;
    if (in.blade < 0.0) {
        c = to_linear(vec3f(0.030, 0.026, 0.012)) * (ambient + 0.6 * sunlight);
    } else {
        let lit = saturate(dot(normal, sun));
        // Sunlight shining through a blade towards the camera glows amber, most at the thin edges.
        let behind = saturate(dot(-normal, sun) + 0.25);
        let towards = pow(saturate(dot(-view, sun) * 0.5 + 0.5), 2.0);
        let through = behind * towards * (0.65 + 0.35 * abs(in.uv.y));
        c = to_linear(vec3f(0.060, 0.055, 0.020)) * (ambient + 0.5 * sunlight + 0.7 * lit * sunlight)
          + to_linear(vec3f(0.80, 0.62, 0.30)) * sunlight * through * 1.6;
        c *= 1.0 - 0.30 * smoothstep_f(0.18, 0.0, abs(in.uv.y));         // the midrib
        c *= 0.92 + 0.08 * sin(in.uv.x * 38.0 + in.blade);               // the wrinkled surface
    }

    // The water between the kelp and the camera; the roots are lost in the gloom near the seabed.
    let seen = through_water(c, CAMERA, -view, distance, s);
    let gloom = in_scatter(CAMERA, -view, distance, s);
    return vec4f(mix(gloom, seen, smoothstep_f(0.0, 9.0, in.lift)), 1.0);
}
