// Caustics: sunlight focused by the waves, from the curvature of the surface. Shared by the water and the
// whales. The including shader declares `curvature` (the ocean's curvature, a 2D array with one layer per
// cascade: d2h/dx2, d2h/dxdz, d2h/dz2, -) and `sea_sampler` (repeat, linear, linear mips).
//
// The approach follows Evan Wallace's WebGL Water (https://github.com/evanw/webgl-water, MIT licence,
// Copyright 2011 Evan Wallace), re-derived for this camera; see THIRD-PARTY-NOTICES.md.

// Curvature of the surface. `broad` keeps only the long waves: seen along a line of sight through the
// water, the fine caustic net averages out and only the broad focusing shows as light shafts.
fn curvature_at(q: vec2f, broad: bool) -> vec4f {
    let c = textureSampleLevel(curvature, sea_sampler, q / cascade_size(0), 0, 0.0);
    return c + textureSampleLevel(curvature, sea_sampler, q / cascade_size(1), 1, select(0.0, 2.5, broad));
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
