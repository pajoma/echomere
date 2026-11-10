/// Metal source for the cove scene, compiled at runtime so the package builds without Xcode.
///
/// The sea is simulated first (an FFT ocean, see `OceanShaders`); then `scene_fragment` draws the picture
/// back to front: the water and the surface seen from below, echoes, whales and fish, kelp, marine snow, and
/// the light shafts. The surface, the shafts and the caustics on the kelp all come from the same wave field,
/// so the light moves with the waves.
///
///
/// The optics (refraction through the wave normal, Snell's window, total internal reflection, and caustics
/// from the curvature of the refracting surface) follow the approach of Evan Wallace's WebGL Water
/// (https://github.com/evanw/webgl-water, MIT licence, Copyright 2011 Evan Wallace), re-derived for this
/// camera; see THIRD-PARTY-NOTICES.md.
enum SceneShaders {
    static let source = """
    #include <metal_stdlib>
    using namespace metal;

    // Must match SceneBuffer.
    constant int TERRAIN = 2;
    constant int CREATURES = 14;
    constant int KELP = 230;
    constant int ECHOES = 326;

    constant float3 WHALE_COLOURS[6] = {
        float3(0.20, 0.25, 0.30),  // slate
        float3(0.11, 0.12, 0.14),  // charcoal
        float3(0.25, 0.22, 0.19),  // grey-brown
        float3(0.40, 0.42, 0.43),  // pale grey
        float3(0.07, 0.10, 0.15),  // blue-black
        float3(0.20, 0.25, 0.23)   // grey-green
    };

    struct VOut { float4 position [[position]]; float2 uv; };

    vertex VOut scene_vertex(uint vid [[vertex_id]]) {
        float2 p = float2(float((vid << 1) & 2), float(vid & 2));
        VOut o;
        o.position = float4(p * 2.0 - 1.0, 0.0, 1.0);
        o.uv = p;
        return o;
    }

    float hash21(float2 p) {
        float3 p3 = fract(float3(p.xyx) * 0.1031);
        p3 += dot(p3, p3.yzx + 33.33);
        return fract((p3.x + p3.y) * p3.z);
    }

    float2 rotate(float2 p, float a) {
        float c = cos(a), s = sin(a);
        return float2(c * p.x + s * p.y, -s * p.x + c * p.y);
    }

    // MARK: Camera and sea

    // The camera stands on the seabed, about 12 m down, and is tilted 15 degrees up, like a webcam on the
    // ground. It is a real perspective camera with a wide lens (70 degree vertical field of view): the lower
    // third of the picture looks into the deep, and Snell's window, where the sky shows through, lies along
    // the top.
    constant float CAMERA_DEPTH = 12.0;
    constant float FOCAL = 0.714;
    constant float PITCH = 0.262;
    constant float HORIZON = 0.309;            // 0.5 - FOCAL * tan(PITCH)
    // Sea.cascadeSizes: long swell, wind sea, ripples.
    constant float CASCADE_SIZES[3] = { 211.3, 39.7, 7.43 };
    constant float WATER_IOR = 1.333;
    constant float MAX_WAVE = 3.0;             // no wave rises or dips further than this, in metres

    // A warm, low sun, 22 degrees high, ahead of the camera.
    constant float3 SUN_IN_AIR = float3(0.10, 0.375, 0.922);
    constant float3 SUN_COLOUR = float3(1.00, 0.66, 0.30);

    struct SeaTextures {
        texture2d_array<float> displacement;   // per cascade: (Dx, height, Dz, dDx/dz)
        texture2d_array<float> derivatives;    // per cascade: (slope x, slope z, dDx/dx, dDz/dz)
        texture2d_array<float> curvature;      // per cascade: (d2h/dx2, d2h/dxdz, d2h/dz2, -)
    };

    constexpr sampler seaSampler(address::repeat, filter::linear, mip_filter::linear);

    float3 viewRay(float2 uv, float aspect) {
        float u = (uv.x - 0.5) * aspect;
        float v = uv.y - 0.5;
        float cp = cos(PITCH), sp = sin(PITCH);
        return normalize(float3(u, v * cp + FOCAL * sp, FOCAL * cp - v * sp));
    }

    // The same sea seen from 4 m above, looking towards the sun: for checking the waves.
    float3 viewRayAbove(float2 uv, float aspect) {
        float u = (uv.x - 0.5) * aspect;
        float v = uv.y - 0.5;
        float cp = cos(-0.14), sp = sin(-0.14);
        return normalize(float3(u, v * cp + FOCAL * sp, FOCAL * cp - v * sp));
    }

    // Direction towards the sun under water (the sunlight bends towards the vertical as it enters).
    float3 sunInWater() {
        float3 down = refract(-normalize(SUN_IN_AIR), float3(0, 1, 0), 1.0 / WATER_IOR);
        return -down;
    }

    // The colour of the water itself, deep blue at the bottom, lighter towards the surface.
    float3 water(float y) {
        float3 deep = float3(0.002, 0.015, 0.055);
        float3 middle = float3(0.010, 0.075, 0.210);
        float3 high = float3(0.030, 0.170, 0.400);
        float3 c = mix(deep, middle, smoothstep(0.05, HORIZON + 0.10, y));
        return mix(c, high, smoothstep(HORIZON + 0.10, 1.0, y));
    }

    // How strongly the waves focus sunlight at `depth` metres below surface point `q`: the inverse of how
    // much a small bundle of refracted sunlight has spread (< 1) or narrowed (> 1) on its way down.
    // The slope of the surface at q (metres), summed over the cascades. Ripples finer than a pixel are faded
    // out with distance; the choppy crests (horizontal displacement) steepen the slope.
    float2 slopeAt(SeaTextures sea, float2 q, float distance) {
        float2 slope = 0.0;
        float jxx = 0.0, jzz = 0.0;
        for (int c = 0; c < 3; c++) {
            float fade = c == 2 ? exp(-distance / 18.0) : (c == 1 ? exp(-distance / 90.0) : 1.0);
            float4 d = sea.derivatives.sample(seaSampler, q / CASCADE_SIZES[c], c, level(0.0));
            slope += d.xy * fade;
            jxx += d.z * fade;
            jzz += d.w * fade;
        }
        return slope / max(float2(1.0 + jxx, 1.0 + jzz), 0.25);
    }

    // Curvature of the surface. `broad` keeps only the long waves: seen along a line of sight through the
    // water, the fine caustic net averages out and only the broad focusing shows as light shafts.
    float4 curvatureAt(SeaTextures sea, float2 q, bool broad) {
        float4 c = sea.curvature.sample(seaSampler, q / CASCADE_SIZES[0], 0, level(0.0));
        if (broad) return c + sea.curvature.sample(seaSampler, q / CASCADE_SIZES[1], 1, level(2.5));
        return c + sea.curvature.sample(seaSampler, q / CASCADE_SIZES[1], 1, level(0.0));
    }

    float focusAt(SeaTextures sea, float2 q, float depth, bool broad) {
        float4 c = curvatureAt(sea, q, broad);
        // The sun is a disk, not a point, so deep caustics blur: the focusing grows more slowly with depth.
        // The broad version already reads blurred curvature, so it needs less of this.
        float blurred = depth / (1.0 + depth / (broad ? 20.0 : 8.0));
        float k = blurred * (1.0 - 1.0 / WATER_IOR);
        float jacobian = (1.0 + k * c.x) * (1.0 + k * c.z) - (k * c.y) * (k * c.y);
        return min(1.0 / max(abs(jacobian), 0.15), 4.0);
    }

    // Caustics on something at `depth` metres: the focused light, following the sun's slant to the surface.
    float causticAt(SeaTextures sea, float2 position, float depth, bool broad) {
        float3 sun = sunInWater();
        float2 q = position + sun.xz * (depth / sun.y);
        return focusAt(sea, q, depth, broad);
    }

    // Height of the long waves at the point x (metres). The choppy crests are waves pushed sideways, so the
    // sideways push is undone first to find which part of the wave lies above x.
    float heightAt(SeaTextures sea, float2 x) {
        float4 d = sea.displacement.sample(seaSampler, x / CASCADE_SIZES[0], 0, level(0.0));
        return sea.displacement.sample(seaSampler, (x - d.xz) / CASCADE_SIZES[0], 0, level(0.0)).y;
    }

    // Where a line of sight meets the waves: the distance along the ray, or a negative value if it never
    // does. Works from below and from above. The ray is walked through the layer the waves occupy until it
    // crosses the surface, then the crossing is refined.
    float surfaceHit(SeaTextures sea, float3 origin, float3 ray) {
        if (abs(ray.y) < 1e-4) return -1.0;
        float side = sign(ray.y);
        float start = max((-MAX_WAVE * side - origin.y) / ray.y, 0.0);
        float end = min((MAX_WAVE * side - origin.y) / ray.y, 600.0);
        if (end <= start) return -1.0;
        const int STEPS = 10;
        float dt = (end - start) / float(STEPS);
        float before = start;
        float gapBefore = side * (origin.y + start * ray.y - heightAt(sea, origin.xz + ray.xz * start));
        for (int i = 1; i <= STEPS; i++) {
            float t = start + dt * float(i);
            float gap = side * (origin.y + t * ray.y - heightAt(sea, origin.xz + ray.xz * t));
            if (gap > 0.0) {
                for (int k = 0; k < 2; k++) {               // refine between the last two steps
                    float middle = before + (t - before) * (-gapBefore / (gap - gapBefore));
                    float gapMiddle = side * (origin.y + middle * ray.y
                                              - heightAt(sea, origin.xz + ray.xz * middle));
                    if (gapMiddle > 0.0) { t = middle; gap = gapMiddle; }
                    else { before = middle; gapBefore = gapMiddle; }
                }
                return before + (t - before) * (-gapBefore / (gap - gapBefore));
            }
            before = t;
            gapBefore = gap;
        }
        return end;
    }

    // The upward normal of the surface above the point x, from all three wave scales.
    float3 normalAt(SeaTextures sea, float2 x, float distance) {
        float2 push = sea.displacement.sample(seaSampler, x / CASCADE_SIZES[0], 0, level(0.0)).xz
                    + sea.displacement.sample(seaSampler, x / CASCADE_SIZES[1], 1, level(0.0)).xz;
        float2 slope = slopeAt(sea, x - push, distance);
        return normalize(float3(-slope.x, 1.0, -slope.y));
    }

    // The sky and the sun, as seen in direction `direction` above the water.
    float3 sky(float3 direction) {
        float sunAngle = saturate(dot(direction, normalize(SUN_IN_AIR)));
        // Evening sky: warm near the horizon and around the sun, blue-grey overhead.
        float3 c = mix(float3(1.00, 0.60, 0.26), float3(0.50, 0.40, 0.38), pow(saturate(direction.y), 0.7));
        c += SUN_COLOUR * (pow(sunAngle, 3000.0) * 60.0 + pow(sunAngle, 200.0) * 3.0 + pow(sunAngle, 10.0) * 0.7);
        return c;
    }

    float3 toneMap(float3 c) { return 1.0 - exp(-c * 1.0); }

    // The underside of the surface, drawn where the line of sight really meets the waves. Inside Snell's
    // window the sky and the sun shine through; outside it the surface is a mirror for the water below. Every
    // wave tilts the surface, so the edge of the window, the sun's glitter and the mirrored water all move with
    // the waves.
    float3 surfaceUnderside(SeaTextures sea, float3 ray, float y) {
        float3 body = water(y);
        if (ray.y <= 0.0) return body;
        float3 origin = float3(0.0, -CAMERA_DEPTH, 0.0);
        float t = surfaceHit(sea, origin, ray);
        if (t < 0.0) return body;
        float3 hit = origin + ray * t;
        float3 normal = normalAt(sea, hit.xz, t);

        // The mirror shows the water below: lighter near the surface, dark blue straight down.
        float3 mirrored = reflect(ray, -normal);
        float3 reflected = mix(float3(0.004, 0.035, 0.110), float3(0.030, 0.160, 0.340),
                               pow(1.0 - saturate(-mirrored.y), 3.0));
        float3 through = refract(ray, -normal, WATER_IOR);
        float3 c;
        if (dot(through, through) < 1e-5) {
            c = reflected;                                  // total internal reflection
        } else {
            float fresnel = 0.02 + 0.98 * pow(1.0 - saturate(dot(through, normal)), 5.0);
            c = mix(sky(through), reflected, fresnel);
        }
        c = toneMap(c);
        // Light reaching the camera through the water turns blue and fades with distance.
        float3 clear = exp(-t * float3(0.030, 0.022, 0.016));
        return c * clear + body * (1.0 - clear);
    }

    // The sea from above, for checking the waves against footage of a real sea.
    float3 seaFromAbove(SeaTextures sea, float3 ray) {
        float3 origin = float3(0.0, 4.0, 0.0);
        float3 horizon = toneMap(sky(normalize(float3(ray.x, max(ray.y, 0.0) + 0.02, ray.z))));
        if (ray.y >= 0.0) return horizon;
        float t = surfaceHit(sea, origin, ray);
        if (t < 0.0) return horizon;
        float3 hit = origin + ray * t;
        float3 normal = normalAt(sea, hit.xz, t);
        float3 mirrored = reflect(ray, normal);
        mirrored.y = abs(mirrored.y);
        float fresnel = 0.02 + 0.98 * pow(1.0 - saturate(dot(-ray, normal)), 5.0);
        float3 c = toneMap(mix(float3(0.010, 0.060, 0.090), sky(mirrored), fresnel));
        return mix(horizon, c, exp(-t / 900.0));
    }

    // Light shafts: sunlight scattered towards the camera along its line of sight. At every step the light is
    // as bright as the long waves above have focused it, so the shafts sway and flicker with the waves; a shaft
    // is a line through the water along the sunlight, so collecting along the line of sight draws it as a streak.
    // Seawater scatters mostly forwards, so the shafts are strongest when looking towards the sun.
    float3 lightShafts(SeaTextures sea, float3 ray, float2 pixel, float y) {
        float3 sun = sunInWater();
        float g = 0.55;
        float mu = dot(ray, sun);
        float phase = (1.0 - g * g) / pow(1.0 + g * g - 2.0 * g * mu, 1.5);
        const int STEPS = 24;
        float range = ray.y > 0.0 ? min(32.0, (CAMERA_DEPTH - 0.3) / ray.y) : 32.0;
        float dt = range / float(STEPS);
        // A fixed per-pixel offset hides the steps without flickering from frame to frame.
        float offset = hash21(floor(pixel));
        float light = 0.0;
        for (int i = 0; i < STEPS; i++) {
            float t = (float(i) + offset) * dt;
            float3 point = float3(0.0, -CAMERA_DEPTH, 0.0) + ray * t;
            float depth = -point.y;
            if (depth < 0.3) continue;
            float focus = causticAt(sea, point.xz, depth, true);
            light += ((focus - 1.0) * 1.8 + 0.10) * exp(-depth * 0.07) * exp(-t * 0.05) * dt;
        }
        // Only very few rays reach the deep in the lower third of the picture.
        float reach = mix(0.12, 1.0, smoothstep(0.08, HORIZON + 0.22, y));
        // The light is warm just under the surface and turns blue on its way down.
        float3 tint = mix(float3(0.030, 0.085, 0.180), float3(0.170, 0.115, 0.060), smoothstep(HORIZON, 1.0, y));
        return tint * 0.09 * max(light, 0.0) * min(phase, 6.0) * reach;
    }

    float3 fogged(float3 c, float depth, float y) {
        return mix(c, water(y), saturate(0.12 + 0.78 * depth));
    }

    float4 drawWhale(float2 p, float4 a, float4 b, float4 c, float px) {
        float scale = 0.30 * b.y * (1.15 - 0.65 * a.z);
        float2 q = (p - a.xy) / scale;
        if (abs(q.x) > 0.75 || abs(q.y) > 0.35) return float4(0.0);
        q.x /= sign(a.w + 1e-4) * max(abs(a.w), 0.18);
        float aa = px / scale * 1.2;

        float u = q.x + 0.5;                       // 0 at the tail stock, 1 at the snout
        float beat = sin(b.x);
        float spine = beat * 0.06 * pow(saturate(1.0 - u), 2.4);
        float girth = pow(saturate(sin(3.14159 * pow(saturate(u), 1.25))), 0.6);
        float thickTop = 0.018 * (1.0 - u) + 0.115 * girth
                       + 0.022 * exp(-pow((u - 0.30) / 0.035, 2.0));      // small dorsal fin
        float thickBottom = 0.016 * (1.0 - u) + 0.105 * girth;
        float dy = q.y - spine;
        float dBody = max(max(dy - thickTop, -dy - thickBottom), max(-u - 0.01, u - 1.0));

        float2 fq = rotate(q - float2(-0.54, beat * 0.06), beat * 0.4);
        float dFluke = (length(fq / float2(0.10, 0.018)) - 1.0) * 0.018;

        float2 pq = rotate(q - float2(0.16, spine - 0.085), 0.75 + 0.06 * sin(b.x * 0.5));
        float dFin = (length(pq / float2(0.14, 0.02)) - 1.0) * 0.02;

        float dist = min(dBody, min(dFluke, dFin));
        float alpha = smoothstep(aa, -aa, dist) * b.z;
        if (alpha <= 0.001) return float4(0.0);

        float3 base = WHALE_COLOURS[clamp(int(c.x), 0, 5)];
        float side = saturate(dy / max(dy > 0.0 ? thickTop : thickBottom, 0.012) * 0.5 + 0.5); // 0 belly, 1 back
        float3 col = mix(base * 1.6 + 0.02, base * 0.8, smoothstep(0.2, 0.6, side));

        int pattern = int(b.w);
        if (pattern == 1) {            // mottled
            float2 g = q * float2(26.0, 34.0);
            float2 cell = floor(g);
            float h = hash21(cell);
            float2 jitter = float2(hash21(cell + 1.7), hash21(cell + 9.2)) - 0.5;
            float radius = 0.12 + 0.18 * hash21(cell + 4.4);
            float spot = smoothstep(radius, radius - 0.08, length(fract(g) - 0.5 - jitter * 0.5));
            col = mix(col, col * 1.45 + 0.02, spot * step(0.62, h) * 0.55);
        } else if (pattern == 2) {     // scars
            float l = abs(fract((q.x + q.y * 2.2) * 9.0 + hash21(floor(q * 6.0)) * 0.5) - 0.5);
            col = mix(col, col * 1.6 + 0.03,
                      smoothstep(0.04, 0.0, l) * 0.5 * step(0.62, hash21(floor(q * 5.0))) * step(0.3, u));
        } else if (pattern == 3) {     // pale belly
            col = mix(col, float3(0.50, 0.52, 0.51), (1.0 - smoothstep(0.15, 0.45, side)) * 0.7);
        }

        col += float3(0.05, 0.10, 0.11) * smoothstep(0.8, 1.0, side) * 0.6;   // light from above
        col *= mix(1.0, 0.25, smoothstep(0.014, 0.007, length(q - float2(0.80, spine - 0.01))));  // eye
        float mouth = abs(q.y - (spine - 0.035 - 0.05 * (1.0 - u))) ;
        col *= mix(1.0, 0.6, smoothstep(0.006, 0.0, mouth) * smoothstep(0.62, 0.72, u) * step(u, 0.99)); // mouth line
        return float4(fogged(col, a.z, a.y), alpha);
    }

    float4 drawFish(float2 p, float4 a, float4 b, float4 c, float px) {
        float scale = 0.024 * b.y * (1.15 - 0.6 * a.z);
        float2 q = (p - a.xy) / scale;
        if (abs(q.x) > 1.0 || abs(q.y) > 0.5) return float4(0.0);
        q.x /= sign(a.w + 1e-4) * max(abs(a.w), 0.2);
        float aa = px / scale * 1.2;

        float dBody = (length(q / float2(0.5, 0.15)) - 1.0) * 0.15;
        float2 tq = q - float2(-0.5, sin(b.x) * 0.05);
        float tailHalf = 0.03 + (-tq.x) * 0.55;
        float dTail = max(abs(tq.y) - tailHalf, max(tq.x - 0.04, -tq.x - 0.22));
        float alpha = smoothstep(aa, -aa, min(dBody, dTail)) * b.z;
        if (alpha <= 0.001) return float4(0.0);

        float3 silver = float3(0.24, 0.31, 0.35);
        float3 gold = float3(0.34, 0.31, 0.19);
        float3 col = mix(silver, gold, saturate(c.x / 3.0));
        col *= 0.7 + 0.5 * saturate(q.y / 0.15 * 0.5 + 0.5);
        return float4(fogged(col, a.z, a.y), alpha);
    }

    float4 drawKelp(float2 p, float4 a, float4 b, float px, float t, SeaTextures sea) {
        float s = (p.y - a.y) / a.z;
        if (s < 0.0 || s > 1.0) return float4(0.0);
        float xc = a.x + b.x * s * s + 0.006 * sin(s * 9.0 + b.y + t * 0.3);
        float w = b.z * (1.0 - 0.55 * s) * (1.0 + 1.1 * max(0.0, sin(s * 47.0 + b.y * 3.0)));
        float alpha = smoothstep(px, -px, abs(p.x - xc) - w) * smoothstep(1.0, 0.92, s);
        if (alpha <= 0.001) return float4(0.0);
        float3 col = mix(float3(0.030, 0.040, 0.018), float3(0.10, 0.12, 0.04), s);
        col += float3(0.02, 0.04, 0.01) * max(causticAt(sea, float2(p.x * 14.0, a.w * 30.0), 8.0 + (1.0 - p.y) * 6.0, false) - 0.5, 0.0) * s;
        return float4(fogged(col, a.w, p.y), alpha);
    }

    float3 drawEcho(float2 p, float4 a, float4 b, float t, int i) {
        float r = 0.06 * b.y * (1.15 - 0.6 * a.z);
        float2 rel = p - a.xy;
        float dd = length(rel / float2(1.8, 1.0));
        float shimmer = a.w * exp(-dd * dd / (r * r))
                      * (0.55 + 0.45 * sin(t * 0.6 + float(i) * 2.1 + p.x * 40.0 + p.y * 25.0));
        float3 c = float3(0.10, 0.20, 0.24) * shimmer * 0.6;
        if (b.x >= 0.0) {
            float radius = b.x * 0.28;
            float ring = exp(-pow((length(rel) - radius) / 0.006, 2.0));
            c += float3(0.12, 0.22, 0.26) * ring * (1.0 - b.x) * a.w * 0.5;
        }
        return c;
    }

    fragment float4 scene_fragment(VOut in [[stage_in]], constant float4 *d [[buffer(0)]],
                                   texture2d_array<float> displacement [[texture(0)]],
                                   texture2d_array<float> derivatives [[texture(1)]],
                                   texture2d_array<float> curvature [[texture(2)]]) {
        SeaTextures sea = { displacement, derivatives, curvature };
        float aspect = d[0].x;
        float t = d[0].y;
        int creatureCount = int(d[0].z);
        int kelpCount = int(d[0].w);
        int echoCount = int(d[1].x);
        float px = d[1].y;
        float2 p = float2(0.8 + (in.uv.x - 0.5) * aspect, in.uv.y);

        float3 ray = viewRay(in.uv, aspect);
        if (d[1].z > 0.5) {
            return float4(seaFromAbove(sea, viewRayAbove(in.uv, aspect)), 1.0);
        }
        float3 col = surfaceUnderside(sea, ray, p.y);


        for (int i = 0; i < echoCount; i++) {
            col += drawEcho(p, d[ECHOES + i * 2], d[ECHOES + i * 2 + 1], t, i);
        }


        for (int i = 0; i < creatureCount; i++) {
            int b = CREATURES + i * 3;
            float4 c = d[b + 2];
            float4 s = c.y < 0.5 ? drawWhale(p, d[b], d[b + 1], c, px) : drawFish(p, d[b], d[b + 1], c, px);
            col = mix(col, s.rgb, s.a);
        }

        for (int i = 0; i < kelpCount; i++) {
            float4 k = drawKelp(p, d[KELP + i * 2], d[KELP + i * 2 + 1], px, t, sea);
            col = mix(col, k.rgb, k.a);
        }


        // Marine snow, drifting down.
        for (int l = 0; l < 2; l++) {
            float fl = float(l);
            float sc = l == 0 ? 60.0 : 32.0;
            float2 g = p * sc + float2(sin(t * 0.05 + fl) * 2.0, t * (0.012 + 0.01 * fl) * sc);
            float2 cell = floor(g);
            float2 f = fract(g) - 0.5;
            float h = hash21(cell + fl * 17.0);
            float2 off = float2(hash21(cell + 3.1), hash21(cell + 7.7)) - 0.5;
            float m = smoothstep(0.07 + 0.04 * fl, 0.0, length(f - off * 0.6)) * step(0.86, h);
            col += float3(0.05, 0.07, 0.07) * m * (0.5 + 0.5 * fl);
        }

        col += lightShafts(sea, ray, in.position.xy, in.uv.y);

        float2 v = (in.uv - 0.5) * float2(1.0, 0.8);
        col *= 1.0 - 0.35 * dot(v, v);
        col += (hash21(in.position.xy) - 0.5) / 255.0;
        return float4(saturate(col), 1.0);
    }
    """
}
