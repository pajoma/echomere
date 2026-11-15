// The small fish, drawn last over the whole picture: they swim near the camera, in front of the whales and
// the kelp. One full-screen triangle reads every fish from the scene buffer (scene_buffer.wgsl). Port of
// `drawFish` and `fish_fragment` from SceneShaders.swift, lit by the water model of common.wgsl. It writes
// linear light premultiplied by the fish's coverage, blended with (one, one - alpha); `finish()` (and with
// it the vignette) runs in the composite pass.

// How far away the fish swim (lighting design), in metres: a fish at depth 0 is this near, at depth 1 this
// far.
const FISH_NEAR: f32 = 3.0;
const FISH_FAR: f32 = 25.0;

// One fish at picture point `p` (x from 0.8 - aspect / 2, y 0..1 up): its linear colour and its coverage.
// a: x, y, depth, facing · b: tail phase, size, opacity, - · c: colour, -, -, -. `ray` is the pixel's line
// of sight, `px` the height of a pixel.
fn draw_fish(p: vec2f, a: vec4f, b: vec4f, c: vec4f, px: f32, ray: vec3f, s: Sky) -> vec4f {
    let scale = 0.024 * b.y * (1.15 - 0.6 * a.z);
    var q = (p - a.xy) / scale;
    if (abs(q.x) > 1.0 || abs(q.y) > 0.5) { return vec4f(0.0); }
    q.x /= sign(a.w + 1e-4) * max(abs(a.w), 0.2);
    let aa = px / scale * 1.2;

    let d_body = (length(q / vec2f(0.5, 0.15)) - 1.0) * 0.15;
    let tq = q - vec2f(-0.5, sin(b.x) * 0.05);
    let tail_half = 0.03 + (-tq.x) * 0.55;
    let d_tail = max(abs(tq.y) - tail_half, max(tq.x - 0.04, -tq.x - 0.22));
    let alpha = smoothstep_f(aa, -aa, min(d_body, d_tail)) * b.z;
    if (alpha <= 0.001) { return vec4f(0.0); }

    let silver = to_linear(vec3f(0.24, 0.31, 0.35));
    let gold = to_linear(vec3f(0.27, 0.31, 0.30));
    var col = mix(silver, gold, saturate(c.x / 3.0));
    col *= 0.7 + 0.5 * saturate(q.y / 0.15 * 0.5 + 0.5);          // lighter on the back

    // Lit and fogged by the water model as if the fish were this far along the line of sight.
    let distance = mix(FISH_NEAR, FISH_FAR, a.z);
    let depth = max(CAMERA_DEPTH - distance * ray.y, 0.0);
    col *= ambient_light(depth, vec3f(0.0, 1.0, 0.0), s) + sunlight_at(depth, s) * 0.3;
    return vec4f(through_water(col, CAMERA, ray, distance, s), alpha);
}

@fragment
fn fish_fragment(in: VOut) -> @location(0) vec4f {
    let aspect = d[0].x;
    let fish_count = i32(d[0].z);
    let px = d[1].y;
    let s = sky_from(d[SKY], d[SKY + 1], d[SKY + 2], d[SKY + 3]);
    let p = vec2f(0.8 + (in.uv.x - 0.5) * aspect, in.uv.y);
    let ray = view_ray(in.uv, aspect);
    // The colour is premultiplied by its coverage; later fish in the list cover earlier ones.
    var result = vec4f(0.0);
    for (var i = 0; i < fish_count; i++) {
        let b = CREATURES + i * 3;
        let f = draw_fish(p, d[b], d[b + 1], d[b + 2], px, ray, s);
        result = vec4f(result.rgb * (1.0 - f.a) + f.rgb * f.a, result.a * (1.0 - f.a) + f.a);
    }
    if (result.a <= 0.001) { discard; }
    return result;
}
