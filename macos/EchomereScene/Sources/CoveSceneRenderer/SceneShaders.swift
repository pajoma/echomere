/// Metal source for the cove scene, compiled at runtime so the package builds without Xcode.
/// One full-screen pass, drawn back to front: water, far ridge, echoes, middle ridge, whales and fish,
/// kelp, near ridge, marine snow, light from the surface.
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

    // The camera looks slightly up: the underside of the surface fills the top of the picture in
    // perspective, down to this horizon (in screen height).
    constant float HORIZON = 0.80;
    constant float2 SUN = float2(0.86, 1.10);

    // The colour of the water itself, deep blue at the bottom, lighter towards the surface.
    float3 water(float y) {
        float3 deep = float3(0.002, 0.015, 0.055);
        float3 middle = float3(0.010, 0.075, 0.210);
        float3 high = float3(0.080, 0.330, 0.640);
        float3 c = mix(deep, middle, smoothstep(0.0, 0.62, y));
        return mix(c, high, smoothstep(0.62, 1.0, y));
    }

    float valueNoise(float2 p) {
        float2 i = floor(p);
        float2 f = fract(p);
        float2 u = f * f * (3.0 - 2.0 * f);
        return mix(mix(hash21(i), hash21(i + float2(1, 0)), u.x),
                   mix(hash21(i + float2(0, 1)), hash21(i + float2(1, 1)), u.x), u.y);
    }

    float fbm(float2 p) {
        float v = 0.0, a = 0.5;
        for (int i = 0; i < 4; i++) {
            v += a * valueNoise(p);
            p = float2(p.x * 1.6 - p.y * 1.2, p.x * 1.2 + p.y * 1.6);
            a *= 0.5;
        }
        return v;
    }

    // The underside of the surface: bright patches where sunlight comes through the waves, separated by
    // darker, crinkled wave edges, brightest around the sun and fading into haze towards the horizon.
    float3 surfaceUnderside(float2 p, float t) {
        // How far up the surface band this pixel is, 0 at the horizon and 1 at the top edge.
        float above = (p.y - HORIZON) / (1.0 - HORIZON);
        float haze = smoothstep(0.0, 0.55, above);
        if (above <= 0.0) return water(p.y);
        float distance = 0.30 / (above * 0.54 + 0.02);
        float2 s = float2((p.x - SUN.x) * distance, distance * 1.6) * 3.0;
        float2 drift = float2(t * 0.06, t * 0.04);
        float2 warp = float2(fbm(s * 0.45 + drift), fbm(s * 0.45 - drift.yx + 5.2));
        float n = fbm(s + warp * 2.2 + drift);
        float ridge = 1.0 - abs(fbm(s * 1.9 + warp * 1.6 - drift * 1.3) * 2.0 - 1.0);
        float patches = smoothstep(0.46, 0.66, n) * (1.0 - 0.65 * smoothstep(0.78, 0.96, ridge));
        float ripples = smoothstep(0.55, 0.95, ridge);

        float2 toSun = (p - SUN) * float2(0.8, 2.2);
        float sun = exp(-dot(toSun, toSun) / 0.24);

        float3 c = water(p.y) * (0.82 + 0.25 * ripples);
        c += float3(0.20, 0.45, 0.75) * patches * (0.18 + 0.9 * sun);
        c += float3(0.75, 0.90, 1.00) * smoothstep(0.55, 1.0, patches) * sun * 1.5;
        return mix(water(p.y), c, haze);
    }

    // Shafts of light fanning out from the sun, soft and many.
    float3 lightShafts(float2 p, float t) {
        float2 d = p - SUN;
        float angle = atan2(d.x, -d.y);
        float rays = 0.5 + 0.5 * sin(angle * 61.0 + t * 0.05 + 1.5 * sin(angle * 7.0 + t * 0.03));
        rays *= 0.5 + 0.5 * sin(angle * 37.0 - t * 0.04 + 2.0);
        rays *= 0.6 + 0.4 * sin(angle * 13.0 + t * 0.02);
        // The rays reach all the way down to the seabed, fading only slowly with depth.
        float reach = mix(0.45, 1.0, exp(-pow(length(d) / 0.9, 2.0)));
        float cone = exp(-angle * angle / 0.30);
        float belowSurface = mix(1.0, 0.35, smoothstep(HORIZON - 0.05, 1.0, p.y));
        return float3(0.05, 0.15, 0.28) * rays * reach * cone * belowSurface;
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

    float caustic(float2 p, float t) {
        float2 q = p * float2(14.0, 22.0);
        float v = 0.0;
        for (int i = 0; i < 3; i++) {
            float fi = float(i);
            q += float2(sin(q.y * 0.9 + t * 0.35 + fi * 1.7), cos(q.x * 0.8 - t * 0.30 + fi * 2.3));
            v += 1.0 / (1.0 + 8.0 * abs(sin(q.x) * sin(q.y)));
        }
        v /= 3.0;
        return v * v * v;
    }

    float3 drawRidge(float3 col, constant float4 *d, int layer, float2 p, float px, float t,
                     float3 rock, float causticStrength) {
        float h = terrainHeight(d, layer, p.x);
        float cover = smoothstep(px, -px, p.y - h);
        if (cover <= 0.0) return col;
        float lit = smoothstep(h - 0.10, h, p.y);
        float3 c = rock * (0.45 + 0.55 * lit);
        c += float3(0.05, 0.11, 0.12) * caustic(p, t) * causticStrength * lit;
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

    float4 drawKelp(float2 p, float4 a, float4 b, float px, float t) {
        float s = (p.y - a.y) / a.z;
        if (s < 0.0 || s > 1.0) return float4(0.0);
        float xc = a.x + b.x * s * s + 0.006 * sin(s * 9.0 + b.y + t * 0.3);
        float w = b.z * (1.0 - 0.55 * s) * (1.0 + 1.1 * max(0.0, sin(s * 47.0 + b.y * 3.0)));
        float alpha = smoothstep(px, -px, abs(p.x - xc) - w) * smoothstep(1.0, 0.92, s);
        if (alpha <= 0.001) return float4(0.0);
        float3 col = mix(float3(0.030, 0.040, 0.018), float3(0.10, 0.12, 0.04), s);
        col += float3(0.03, 0.05, 0.01) * caustic(p, t) * s;
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

    fragment float4 scene_fragment(VOut in [[stage_in]], constant float4 *d [[buffer(0)]]) {
        float aspect = d[0].x;
        float t = d[0].y;
        int creatureCount = int(d[0].z);
        int kelpCount = int(d[0].w);
        int echoCount = int(d[1].x);
        float px = d[1].y;
        float2 p = float2(0.8 + (in.uv.x - 0.5) * aspect, in.uv.y);

        float3 col = surfaceUnderside(p, t);

        col = drawRidge(col, d, 0, p, px, t, float3(0.020, 0.060, 0.120), 0.2);

        for (int i = 0; i < echoCount; i++) {
            col += drawEcho(p, d[ECHOES + i * 2], d[ECHOES + i * 2 + 1], t, i);
        }

        col = drawRidge(col, d, 1, p, px, t, float3(0.030, 0.070, 0.120), 0.5);

        for (int i = 0; i < creatureCount; i++) {
            int b = CREATURES + i * 3;
            float4 c = d[b + 2];
            float4 s = c.y < 0.5 ? drawWhale(p, d[b], d[b + 1], c, px) : drawFish(p, d[b], d[b + 1], c, px);
            col = mix(col, s.rgb, s.a);
        }

        for (int i = 0; i < kelpCount; i++) {
            float4 k = drawKelp(p, d[KELP + i * 2], d[KELP + i * 2 + 1], px, t);
            col = mix(col, k.rgb, k.a);
        }

        col = drawRidge(col, d, 2, p, px, t, float3(0.060, 0.090, 0.120), 0.8);

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

        col += lightShafts(p, t);

        float2 v = (in.uv - 0.5) * float2(1.0, 0.8);
        col *= 1.0 - 0.35 * dot(v, v);
        col += (hash21(in.position.xy) - 0.5) / 255.0;
        return float4(saturate(col), 1.0);
    }
    """
}
