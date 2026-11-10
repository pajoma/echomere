/// Metal source for the cove scene, compiled at runtime so the package builds without Xcode.
///
/// The sea is simulated first (an FFT ocean, see `OceanShaders`); then `scene_fragment` draws the picture back to front: water and the surface seen from below, far ridge,
///    echoes, middle ridge, whales and fish, kelp, near ridge, marine snow, and the light shafts.
///    The surface, the shafts and the caustics on the seabed all come from the same wave field, so the
///    light moves with the waves.
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

    // The camera is a few metres below the surface and looks almost level: the underside of the surface
    // fills the picture above the horizon at 80 % of the screen height.
    constant float CAMERA_DEPTH = 6.0;
    constant float FOCAL = 1.0;
    constant float PITCH = -0.2915;            // -atan(0.3): puts the horizon at 80 % of the height
    constant float HORIZON = 0.80;
    // Sea.cascadeSizes: long swell, wind sea, ripples.
    constant float CASCADE_SIZES[3] = { 211.3, 39.7, 7.43 };
    constant float WATER_IOR = 1.333;
    // Seen this flat, every ray would be totally reflected. The surface is drawn as if the camera looked
    // up more steeply, so the sunlit Snell's window shows at the top of the picture.
    constant float ELEVATION_STRETCH = 7.0;
    // Snell's window magnifies the waves seen from below; the slopes are gently exaggerated for the shading
    // of the surface only (not for the light focusing).
    constant float SLOPE_GAIN = 1.4;

    // The sun stands about 38 degrees high, ahead of the camera, so its light comes through the top of the picture.
    constant float3 SUN_IN_AIR = float3(0.05, 0.61, 0.79);

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

    // Direction towards the sun under water (the sunlight bends towards the vertical as it enters).
    float3 sunInWater() {
        float3 down = refract(-normalize(SUN_IN_AIR), float3(0, 1, 0), 1.0 / WATER_IOR);
        return -down;
    }

    // The colour of the water itself, deep blue at the bottom, lighter towards the surface.
    float3 water(float y) {
        float3 deep = float3(0.002, 0.015, 0.055);
        float3 middle = float3(0.010, 0.075, 0.210);
        float3 high = float3(0.080, 0.330, 0.640);
        float3 c = mix(deep, middle, smoothstep(0.0, 0.62, y));
        return mix(c, high, smoothstep(0.62, 1.0, y));
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

    // The underside of the surface. Inside Snell's window the sky and the sun shine through the waves;
    // outside it the surface is a mirror for the deep water below. Every wave changes where that boundary
    // falls, so light patches form, travel and break up with the waves.
    float3 surfaceUnderside(SeaTextures sea, float3 ray, float y) {
        float3 deep = water(y);
        if (ray.y <= 0.0) return deep;
        float2 across = normalize(ray.xz);
        float elevation = min(asin(ray.y) * ELEVATION_STRETCH, 1.45);
        float3 incoming = float3(across.x * cos(elevation), sin(elevation), across.y * cos(elevation));
        float distance = CAMERA_DEPTH / sin(elevation);
        float2 q = incoming.xz * distance;
        float2 slope = slopeAt(sea, q, distance) * SLOPE_GAIN;
        float3 normal = normalize(float3(-slope.x, 1.0, -slope.y));

        // Outside Snell's window the surface mirrors the water below: facets tilted to see the lighter water
        // just under the surface look brighter than facets that see the dark depth.
        float3 mirrored = reflect(incoming, -normal);
        float3 reflected = mix(float3(0.006, 0.045, 0.130), float3(0.040, 0.190, 0.380),
                               pow(1.0 - saturate(-mirrored.y * 1.6), 2.0));
        float3 through = refract(incoming, -normal, WATER_IOR);
        float3 c;
        if (dot(through, through) < 1e-5) {
            c = reflected;                                  // total internal reflection
        } else {
            float cosine = saturate(dot(incoming, normal));
            float fresnel = 0.02 + 0.98 * pow(1.0 - saturate(dot(through, normal)), 5.0);
            float sunAngle = saturate(dot(through, normalize(SUN_IN_AIR)));
            float3 sky = mix(float3(0.10, 0.30, 0.58), float3(0.30, 0.55, 0.85), saturate(through.y));
            float3 sun = float3(1.0, 0.97, 0.90) * (pow(sunAngle, 250.0) * 6.0 + pow(sunAngle, 25.0) * 0.8);
            c = mix(sky + sun, reflected, saturate(fresnel + (1.0 - cosine) * 0.15));
        }
        // Light reaching the camera through the water turns blue and fades with distance.
        float3 absorption = exp(-distance * float3(0.11, 0.028, 0.015));
        c = c * absorption + deep * (1.0 - absorption);
        float haze = smoothstep(0.0, 0.025, ray.y);
        return mix(deep, c, haze);
    }

    // Light shafts: sunlight scattered towards the camera along its line of sight. At every step the light is
    // as bright as the long waves above have focused it, so the shafts sway and flicker with the waves; a shaft
    // is a line through the water along the sunlight, so collecting along the line of sight draws it as a streak.
    // Seawater scatters mostly forwards, so the shafts are strongest when looking towards the sun.
    float3 lightShafts(SeaTextures sea, float3 ray, float2 pixel) {
        float3 sun = sunInWater();
        float g = 0.55;
        float mu = dot(ray, sun);
        float phase = (1.0 - g * g) / pow(1.0 + g * g - 2.0 * g * mu, 1.5);
        const int STEPS = 16;
        float range = ray.y > 0.0 ? min(32.0, (CAMERA_DEPTH - 0.3) / ray.y) : 32.0;
        float dt = range / float(STEPS);
        // A fixed per-pixel offset hides the steps without flickering from frame to frame.
        float2 cell = floor(pixel);
        float offset = fract(52.9829189 * fract(0.06711056 * cell.x + 0.00583715 * cell.y));
        float light = 0.0;
        for (int i = 0; i < STEPS; i++) {
            float t = (float(i) + offset) * dt;
            float3 point = float3(0.0, -CAMERA_DEPTH, 0.0) + ray * t;
            float depth = -point.y;
            if (depth < 0.3) continue;
            float focus = causticAt(sea, point.xz, depth, true);
            light += ((focus - 1.0) * 2.4 + 0.10) * exp(-depth * 0.07) * exp(-t * 0.05) * dt;
        }
        return float3(0.035, 0.120, 0.220) * 0.45 * max(light, 0.0) * phase;
    }

    float3 fogged(float3 c, float depth, float y) {
        return mix(c, water(y), saturate(0.12 + 0.78 * depth));
    }

    // Same formula as TerrainLayer.height(at:).
    float terrainHeight(constant float4 *d, int layer, float x) {
        int b = TERRAIN + layer * 4;
        float4 a = d[b + 1], f = d[b + 2], ph = d[b + 3];
        return d[b].x + a.x * sin(f.x * x + ph.x) + a.y * sin(f.y * x + ph.y)
                      + a.z * sin(f.z * x + ph.z) + a.w * sin(f.w * x + ph.w);
    }

    float3 drawRidge(float3 col, constant float4 *d, int layer, float2 p, float px, SeaTextures sea,
                     float3 rock, float causticStrength) {
        float h = terrainHeight(d, layer, p.x);
        float cover = smoothstep(px, -px, p.y - h);
        if (cover <= 0.0) return col;
        float lit = smoothstep(h - 0.10, h, p.y);
        float3 c = rock * (0.45 + 0.55 * lit);
        float layerDepth = d[TERRAIN + layer * 4].y;
        float seabed = 10.0 + 6.0 * layerDepth;
        float caustic = causticAt(sea, float2(p.x * 6.0, layerDepth * 20.0 + (h - p.y) * 8.0), seabed, false);
        c += float3(0.06, 0.14, 0.17) * max(caustic - 0.8, 0.0) * causticStrength * lit;
        c = fogged(c, d[TERRAIN + layer * 4].y, p.y);
        return mix(col, c, cover);
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
        float3 col = surfaceUnderside(sea, ray, p.y);

        col = drawRidge(col, d, 0, p, px, sea, float3(0.020, 0.060, 0.120), 0.2);

        for (int i = 0; i < echoCount; i++) {
            col += drawEcho(p, d[ECHOES + i * 2], d[ECHOES + i * 2 + 1], t, i);
        }

        col = drawRidge(col, d, 1, p, px, sea, float3(0.030, 0.070, 0.120), 0.5);

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

        col = drawRidge(col, d, 2, p, px, sea, float3(0.060, 0.090, 0.120), 0.8);

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

        col += lightShafts(sea, ray, in.position.xy);

        float2 v = (in.uv - 0.5) * float2(1.0, 0.8);
        col *= 1.0 - 0.35 * dot(v, v);
        col += (hash21(in.position.xy) - 0.5) / 255.0;
        return float4(saturate(col), 1.0);
    }
    """
}
