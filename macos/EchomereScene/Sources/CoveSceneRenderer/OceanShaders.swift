/// Metal source for the sea surface: a Tessendorf FFT ocean.
///
/// Ported from web-ocean by Maksymilian Dendura (https://github.com/Noniv/web-ocean, `js/wgsl/sim.js`),
/// MIT licence, Copyright (c) 2026 Maksymilian Dendura; see THIRD-PARTY-NOTICES.md. Changes for Echomere:
/// translated from WGSL to Metal, three cascades instead of six, no foam, and an extra pass that derives the
/// curvature of the surface for the light focusing (caustics and light shafts).
///
/// Passes:
/// - `ocean_init` (once): the initial spectrum h0(k) for every cascade: a fetch-limited JONSWAP wind sea with
///   Donelan-Banner directional spreading, plus a narrow, long-crested swell.
/// - `ocean_evolve` (every frame): h(k, t) and its derivatives in frequency space. Frequencies are quantised to
///   the loop period, so the sea repeats seamlessly.
/// - `ocean_fft_rows`, `ocean_fft_cols` (every frame): radix-2 inverse FFTs in threadgroup memory, writing
///   displacement (Dx, height, Dz, dDx/dz) and derivatives (slope x, slope z, dDx/dx, dDz/dz).
/// - `ocean_curvature` (every frame): the second derivatives of the height, from the slopes.
enum OceanShaders {
    static let cascades = 3
    static let size = 256

    static let source = """
    #include <metal_stdlib>
    using namespace metal;

    #define N 256u
    #define LOGN 8u
    constant float PI_F = 3.14159265358979;
    constant float GRAV = 9.81;
    constant float SURF = 7.4e-5;               // surface tension / density

    // Must match OceanSettings in CoveSceneRenderer.swift.
    struct OceanSettings {
        float4 L;                               // patch size per cascade (m), w unused
        float4 kMin;
        float4 kMax;
        float4 wind;                            // U10 m/s, direction rad, fetch m, JONSWAP gamma
        float4 swell;                           // Hs m, direction rad, peak omega, spreading exponent
        float4 misc;                            // choppiness, short-wave cut (m), loop period s, seed
        float4 timeP;                           // time (looped), -, -, spectrum gain
    };

    float2 waveVec(uint x, uint y, float L) {
        int nx = x >= N / 2u ? int(x) - int(N) : int(x);
        int ny = y >= N / 2u ? int(y) - int(N) : int(y);
        return float2(float(nx), float(ny)) * (2.0 * PI_F / L);
    }
    float omegaOf(float k) { return sqrt(GRAV * k + SURF * k * k * k); }
    float dOmegaOf(float k) { return (GRAV + 3.0 * SURF * k * k) / (2.0 * omegaOf(k)); }
    float2 cmul(float2 a, float2 b) { return float2(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x); }
    float2 mulI(float2 a) { return float2(-a.y, a.x); }

    uint pcgHash(uint v) {
        uint s = v * 747796405u + 2891336453u;
        uint w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
        return (w >> 22u) ^ w;
    }
    float rand01(uint s) { return (float(pcgHash(s) >> 8u) + 0.5) / 16777216.0; }
    float2 gauss2(uint seed) {
        float u1 = rand01(seed * 2u + 17u);
        float u2 = rand01(seed * 2u + 18u + pcgHash(seed));
        float r = sqrt(-2.0 * log(u1));
        float a = 2.0 * PI_F * u2;
        return r * float2(cos(a), sin(a));
    }
    float wrapAngle(float a) { return a - 2.0 * PI_F * floor((a + PI_F) / (2.0 * PI_F)); }

    float jonswap(float w, float wp, float alpha, float gamma) {
        float sigma = w <= wp ? 0.07 : 0.09;
        float d = (w - wp) / (sigma * wp);
        float r = exp(-0.5 * d * d);
        float q = wp / w;
        float q2 = q * q;
        return alpha * GRAV * GRAV / (w * w * w * w * w) * exp(-1.25 * q2 * q2) * pow(gamma, r);
    }
    // Donelan-Banner directional spreading (normalised over [-pi, pi]).
    float donelanBanner(float theta, float w, float wp) {
        float ratio = w / wp;
        float beta;
        if (ratio < 0.95) {
            beta = 2.61 * pow(ratio, 1.3);
        } else if (ratio < 1.6) {
            beta = 2.28 * pow(ratio, -1.3);
        } else {
            float eps = -0.4 + 0.8393 * exp(-0.567 * log(ratio * ratio));
            beta = pow(10.0, eps);
        }
        float sh = 1.0 / cosh(clamp(beta * theta, -20.0, 20.0));
        return beta / (2.0 * tanh(beta * PI_F)) * sh * sh;
    }
    // cos^2s(theta/2) spreading, normalised.
    float cosSpread(float theta, float s) {
        float q = sqrt(s + 0.25) / (2.0 * sqrt(PI_F));
        return q * pow(abs(cos(theta * 0.5)), 2.0 * s);
    }

    float spectrum(float2 k, constant OceanSettings &S) {
        float kl = length(k);
        float w = omegaOf(kl);
        float theta = atan2(k.y, k.x);
        float U = S.wind.x;
        float F = S.wind.z;
        float Sw = 0.0;
        if (U > 0.1) {                          // fetch-limited wind sea (JONSWAP)
            float alpha = 0.076 * pow(U * U / (F * GRAV), 0.22);
            float wp = 22.0 * pow(GRAV * GRAV / (U * F), 1.0 / 3.0);
            Sw = jonswap(w, wp, alpha, S.wind.w) * donelanBanner(wrapAngle(theta - S.wind.y), w, wp);
        }
        if (S.swell.x > 0.001) {                // swell: narrow, long-crested JONSWAP
            float ws = S.swell.z;
            float gs = 6.0;
            float hs4 = S.swell.x * 0.25;
            float alphaS = hs4 * hs4 * 5.0 * ws * ws * ws * ws / (GRAV * GRAV) * (1.0 - 0.287 * log(gs));
            Sw += jonswap(w, ws, alphaS, gs) * cosSpread(wrapAngle(theta - S.swell.y), S.swell.w);
        }
        float sc = S.misc.y;                    // damp the very shortest waves
        Sw *= exp(-kl * kl * sc * sc);
        return Sw * dOmegaOf(kl) / kl * S.timeP.w;
    }

    float amplitude(float2 k, uint c, constant OceanSettings &S) {
        float kl = length(k);
        if (kl < 1e-5 || kl < S.kMin[c] || kl >= S.kMax[c]) return 0.0;
        float dk = 2.0 * PI_F / S.L[c];
        return sqrt(max(spectrum(k, S), 0.0)) * dk * 0.5;
    }

    kernel void ocean_init(device float4 *h0 [[buffer(0)]],
                           constant OceanSettings &S [[buffer(1)]],
                           uint3 id [[thread_position_in_grid]]) {
        uint c = id.z;
        float2 k = waveVec(id.x, id.y, S.L[c]);
        uint mx = (N - id.x) % N;
        uint my = (N - id.y) % N;
        uint seed = uint(S.misc.w) * 7919u + c * 1000003u;
        float2 a = gauss2(seed + (id.y * N + id.x)) * amplitude(k, c, S);
        float2 b = gauss2(seed + (my * N + mx)) * amplitude(-k, c, S);
        h0[(c * N + id.y) * N + id.x] = float4(a, b.x, -b.y);
    }

    kernel void ocean_evolve(device const float4 *h0 [[buffer(0)]],
                             device float4 *specA [[buffer(1)]],
                             device float4 *specB [[buffer(2)]],
                             constant OceanSettings &S [[buffer(3)]],
                             uint3 id [[thread_position_in_grid]]) {
        uint c = id.z;
        uint idx = (c * N + id.y) * N + id.x;
        float2 k = waveVec(id.x, id.y, S.L[c]);
        float kl = length(k);
        if (kl < 1e-5) {
            specA[idx] = float4(0.0);
            specB[idx] = float4(0.0);
            return;
        }
        float w0 = 2.0 * PI_F / S.misc.z;
        float w = floor(omegaOf(kl) / w0) * w0;
        float ph = w * S.timeP.x;
        float2 e = float2(cos(ph), sin(ph));
        float4 H0 = h0[idx];
        // h0(k) travels along +k: h(k,t) = h0(k) e^{-iwt} + conj(h0(-k)) e^{iwt}
        float2 h = cmul(H0.xy, float2(e.x, -e.y)) + cmul(H0.zw, e);
        float2 ih = mulI(h);
        float lam = S.misc.x;
        float ik = 1.0 / kl;
        float2 Dx = ih * (lam * k.x * ik);
        float2 Dz = ih * (lam * k.y * ik);
        float2 jxz = h * (-lam * k.x * k.y * ik);
        float2 sx = ih * k.x;
        float2 sz = ih * k.y;
        float2 jxx = h * (-lam * k.x * k.x * ik);
        float2 jzz = h * (-lam * k.y * k.y * ik);
        specA[idx] = float4(Dx + mulI(Dz), h + mulI(jxz));
        specB[idx] = float4(sx + mulI(sz), jxx + mulI(jzz));
    }

    uint bitrev(uint i) { return reverse_bits(i) >> (32u - LOGN); }

    // In-place radix-2 inverse FFT over threadgroup memory (input already bit-reversed).
    void butterflies(threadgroup float4 *wa, threadgroup float4 *wb, uint t) {
        for (uint s = 0u; s < LOGN; s++) {
            uint halfSize = 1u << s;
            uint pos = t & (halfSize - 1u);
            uint i0 = ((t >> s) << (s + 1u)) + pos;
            uint i1 = i0 + halfSize;
            float ang = PI_F * float(pos) / float(halfSize);
            float2 w = float2(cos(ang), sin(ang));
            float4 a0 = wa[i0], a1 = wa[i1];
            float4 b0 = wb[i0], b1 = wb[i1];
            float4 ta = float4(cmul(a1.xy, w), cmul(a1.zw, w));
            float4 tb = float4(cmul(b1.xy, w), cmul(b1.zw, w));
            wa[i0] = a0 + ta; wa[i1] = a0 - ta;
            wb[i0] = b0 + tb; wb[i1] = b0 - tb;
            threadgroup_barrier(mem_flags::mem_threadgroup);
        }
    }

    kernel void ocean_fft_rows(device const float4 *inA [[buffer(0)]],
                               device const float4 *inB [[buffer(1)]],
                               device float4 *outA [[buffer(2)]],
                               device float4 *outB [[buffer(3)]],
                               uint3 wg [[threadgroup_position_in_grid]],
                               uint t [[thread_index_in_threadgroup]]) {
        threadgroup float4 wa[N];
        threadgroup float4 wb[N];
        uint base = (wg.z * N + wg.x) * N;
        uint e0 = t;
        uint e1 = t + N / 2u;
        wa[bitrev(e0)] = inA[base + e0];
        wa[bitrev(e1)] = inA[base + e1];
        wb[bitrev(e0)] = inB[base + e0];
        wb[bitrev(e1)] = inB[base + e1];
        threadgroup_barrier(mem_flags::mem_threadgroup);
        butterflies(wa, wb, t);
        outA[base + e0] = wa[e0];
        outA[base + e1] = wa[e1];
        outB[base + e0] = wb[e0];
        outB[base + e1] = wb[e1];
    }

    kernel void ocean_fft_cols(device const float4 *inA [[buffer(0)]],
                               device const float4 *inB [[buffer(1)]],
                               texture2d_array<float, access::write> displacement [[texture(0)]],
                               texture2d_array<float, access::write> derivatives [[texture(1)]],
                               uint3 wg [[threadgroup_position_in_grid]],
                               uint t [[thread_index_in_threadgroup]]) {
        threadgroup float4 wa[N];
        threadgroup float4 wb[N];
        uint x = wg.x;
        uint c = wg.z;
        uint e0 = t;
        uint e1 = t + N / 2u;
        wa[bitrev(e0)] = inA[(c * N + e0) * N + x];
        wa[bitrev(e1)] = inA[(c * N + e1) * N + x];
        wb[bitrev(e0)] = inB[(c * N + e0) * N + x];
        wb[bitrev(e1)] = inB[(c * N + e1) * N + x];
        threadgroup_barrier(mem_flags::mem_threadgroup);
        butterflies(wa, wb, t);
        // a = (Dx, Dz, height, dDx/dz), b = (slope x, slope z, dDx/dx, dDz/dz)
        float4 a0 = wa[e0], a1 = wa[e1];
        displacement.write(float4(a0.x, a0.z, a0.y, a0.w), uint2(x, e0), c);
        displacement.write(float4(a1.x, a1.z, a1.y, a1.w), uint2(x, e1), c);
        derivatives.write(wb[e0], uint2(x, e0), c);
        derivatives.write(wb[e1], uint2(x, e1), c);
    }

    // Second derivatives of the height (d2h/dx2, d2h/dxdz, d2h/dz2) from the slopes, for the light focusing.
    kernel void ocean_curvature(texture2d_array<float, access::read> derivatives [[texture(0)]],
                                texture2d_array<float, access::write> curvature [[texture(1)]],
                                constant OceanSettings &S [[buffer(0)]],
                                uint3 id [[thread_position_in_grid]]) {
        uint c = id.z;
        float spacing = S.L[c] / float(N);
        uint xp = (id.x + 1u) % N, xm = (id.x + N - 1u) % N;
        uint yp = (id.y + 1u) % N, ym = (id.y + N - 1u) % N;
        float4 right = derivatives.read(uint2(xp, id.y), c);
        float4 left = derivatives.read(uint2(xm, id.y), c);
        float4 down = derivatives.read(uint2(id.x, yp), c);
        float4 up = derivatives.read(uint2(id.x, ym), c);
        float hxx = (right.x - left.x) / (2.0 * spacing);
        float hzz = (down.y - up.y) / (2.0 * spacing);
        float hxz = (down.x - up.x) / (2.0 * spacing);
        curvature.write(float4(hxx, hxz, hzz, 0.0), id.xy, c);
    }
    """
}
