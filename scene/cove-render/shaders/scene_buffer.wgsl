// The scene buffer and the full-screen triangle, shared by the full-screen passes (water.wgsl).

// Must match `SceneBuffer` in water.rs.
const SKY: i32 = 2;
const ECHOES: i32 = 14;
const COUNT: i32 = 24;

// d[0]: aspect, shader time (s within the day), -, -
// d[1]: echo count, 1 / pixels high, sea from above (0 or 1), -
// d[SKY..SKY+3]: light direction + intensity, light colour + ambient, zenith + night, horizon + warmth
// d[ECHOES + 2i ..]: x, y, depth, intensity | call (or -1), size, -, -
@group(0) @binding(0) var<uniform> d: array<vec4f, COUNT>;

struct VOut {
    @builtin(position) position: vec4f,
    // 0..1 across the scene target, y up.
    @location(0) uv: vec2f,
};

@vertex
fn fullscreen_vertex(@builtin(vertex_index) vid: u32) -> VOut {
    let p = vec2f(f32((vid << 1u) & 2u), f32(vid & 2u));
    var o: VOut;
    o.position = vec4f(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = p;
    return o;
}
