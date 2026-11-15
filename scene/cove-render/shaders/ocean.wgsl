// The sea surface: a Tessendorf FFT ocean.
//
// Ported from web-ocean by Maksymilian Dendura (https://github.com/Noniv/web-ocean, `js/wgsl/sim.js`),
// MIT licence, Copyright (c) 2026 Maksymilian Dendura; see THIRD-PARTY-NOTICES.md. Changes for Echomere
// (carried over from `OceanShaders.swift`, which this file follows line by line): three cascades instead
// of six, N = 256, no foam, an extra pass that derives the curvature of the surface for the light
// focusing (caustics and light shafts), and a pass that builds the curvature's mipmaps.
//
// Passes:
// - `ocean_init` (once): the initial spectrum h0(k) for every cascade: a fetch-limited JONSWAP wind sea
//   with Donelan-Banner directional spreading, plus a narrow, long-crested swell.
// - `ocean_evolve` (every frame): h(k, t) and its derivatives in frequency space. Frequencies are
//   quantised to the loop period, so the sea repeats seamlessly.
// - `ocean_fft_rows`, `ocean_fft_cols` (every frame): radix-2 inverse FFTs in workgroup memory, writing
//   displacement (Dx, height, Dz, dDx/dz) and derivatives (slope x, slope z, dDx/dx, dDz/dz).
// - `ocean_curvature` (every frame): the second derivatives of the height, from the slopes.
// - `ocean_curvature_mip` (every frame, once per level): a 2x2 average into the next mip level.

const N: u32 = 256u;
const LOGN: u32 = 8u;
const PI_F: f32 = 3.14159265358979;
const GRAV: f32 = 9.81;
const SURF: f32 = 7.4e-5;                       // surface tension / density

// Must match `OceanSettings` in ocean.rs.
struct OceanSettings {
    L: vec4f,                                   // patch size per cascade (m), w unused
    kMin: vec4f,
    kMax: vec4f,
    wind: vec4f,                                // U10 m/s, direction rad, fetch m, JONSWAP gamma
    swell: vec4f,                               // Hs m, direction rad, peak omega, spreading exponent
    misc: vec4f,                                // choppiness, short-wave cut (m), loop period s, seed
    timeP: vec4f,                               // time (looped), -, -, spectrum gain
};

fn waveVec(x: u32, y: u32, L: f32) -> vec2f {
    let nx = select(i32(x), i32(x) - i32(N), x >= N / 2u);
    let ny = select(i32(y), i32(y) - i32(N), y >= N / 2u);
    return vec2f(f32(nx), f32(ny)) * (2.0 * PI_F / L);
}
fn omegaOf(k: f32) -> f32 { return sqrt(GRAV * k + SURF * k * k * k); }
fn dOmegaOf(k: f32) -> f32 { return (GRAV + 3.0 * SURF * k * k) / (2.0 * omegaOf(k)); }
fn cmul(a: vec2f, b: vec2f) -> vec2f { return vec2f(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x); }
fn mulI(a: vec2f) -> vec2f { return vec2f(-a.y, a.x); }

fn pcgHash(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}
fn rand01(s: u32) -> f32 { return (f32(pcgHash(s) >> 8u) + 0.5) / 16777216.0; }
fn gauss2(seed: u32) -> vec2f {
    let u1 = rand01(seed * 2u + 17u);
    let u2 = rand01(seed * 2u + 18u + pcgHash(seed));
    let r = sqrt(-2.0 * log(u1));
    let a = 2.0 * PI_F * u2;
    return r * vec2f(cos(a), sin(a));
}
fn wrapAngle(a: f32) -> f32 { return a - 2.0 * PI_F * floor((a + PI_F) / (2.0 * PI_F)); }

fn jonswap(w: f32, wp: f32, alpha: f32, gamma: f32) -> f32 {
    let sigma = select(0.09, 0.07, w <= wp);
    let d = (w - wp) / (sigma * wp);
    let r = exp(-0.5 * d * d);
    let q = wp / w;
    let q2 = q * q;
    return alpha * GRAV * GRAV / (w * w * w * w * w) * exp(-1.25 * q2 * q2) * pow(gamma, r);
}
// Donelan-Banner directional spreading (normalised over [-pi, pi]).
fn donelanBanner(theta: f32, w: f32, wp: f32) -> f32 {
    let ratio = w / wp;
    var beta: f32;
    if (ratio < 0.95) {
        beta = 2.61 * pow(ratio, 1.3);
    } else if (ratio < 1.6) {
        beta = 2.28 * pow(ratio, -1.3);
    } else {
        let eps = -0.4 + 0.8393 * exp(-0.567 * log(ratio * ratio));
        beta = pow(10.0, eps);
    }
    let sh = 1.0 / cosh(clamp(beta * theta, -20.0, 20.0));
    return beta / (2.0 * tanh(beta * PI_F)) * sh * sh;
}
// cos^2s(theta/2) spreading, normalised.
fn cosSpread(theta: f32, s: f32) -> f32 {
    let q = sqrt(s + 0.25) / (2.0 * sqrt(PI_F));
    return q * pow(abs(cos(theta * 0.5)), 2.0 * s);
}

fn spectrum(k: vec2f) -> f32 {
    let kl = length(k);
    let w = omegaOf(kl);
    let theta = atan2(k.y, k.x);
    let U = S.wind.x;
    let F = S.wind.z;
    var Sw = 0.0;
    if (U > 0.1) {                              // fetch-limited wind sea (JONSWAP)
        let alpha = 0.076 * pow(U * U / (F * GRAV), 0.22);
        let wp = 22.0 * pow(GRAV * GRAV / (U * F), 1.0 / 3.0);
        Sw = jonswap(w, wp, alpha, S.wind.w) * donelanBanner(wrapAngle(theta - S.wind.y), w, wp);
    }
    if (S.swell.x > 0.001) {                    // swell: narrow, long-crested JONSWAP
        let ws = S.swell.z;
        let gs = 6.0;
        let hs4 = S.swell.x * 0.25;
        let alphaS = hs4 * hs4 * 5.0 * ws * ws * ws * ws / (GRAV * GRAV) * (1.0 - 0.287 * log(gs));
        Sw += jonswap(w, ws, alphaS, gs) * cosSpread(wrapAngle(theta - S.swell.y), S.swell.w);
    }
    let sc = S.misc.y;                          // damp the very shortest waves
    Sw *= exp(-kl * kl * sc * sc);
    return Sw * dOmegaOf(kl) / kl * S.timeP.w;
}

fn amplitude(k: vec2f, c: u32) -> f32 {
    let kl = length(k);
    if (kl < 1e-5 || kl < S.kMin[c] || kl >= S.kMax[c]) { return 0.0; }
    let dk = 2.0 * PI_F / S.L[c];
    return sqrt(max(spectrum(k), 0.0)) * dk * 0.5;
}

@group(0) @binding(0) var<uniform> S: OceanSettings;

// ---------------------------------------------------------------- spectrum

@group(0) @binding(1) var<storage, read_write> h0Out: array<vec4f>;

@compute @workgroup_size(16, 16, 1)
fn ocean_init(@builtin(global_invocation_id) id: vec3u) {
    let c = id.z;
    let k = waveVec(id.x, id.y, S.L[c]);
    let mx = (N - id.x) % N;
    let my = (N - id.y) % N;
    let seed = u32(S.misc.w) * 7919u + c * 1000003u;
    let a = gauss2(seed + (id.y * N + id.x)) * amplitude(k, c);
    let b = gauss2(seed + (my * N + mx)) * amplitude(-k, c);
    h0Out[(c * N + id.y) * N + id.x] = vec4f(a, b.x, -b.y);
}

@group(0) @binding(2) var<storage, read> h0: array<vec4f>;
@group(0) @binding(3) var<storage, read_write> specA: array<vec4f>;
@group(0) @binding(4) var<storage, read_write> specB: array<vec4f>;

@compute @workgroup_size(16, 16, 1)
fn ocean_evolve(@builtin(global_invocation_id) id: vec3u) {
    let c = id.z;
    let idx = (c * N + id.y) * N + id.x;
    let k = waveVec(id.x, id.y, S.L[c]);
    let kl = length(k);
    if (kl < 1e-5) {
        specA[idx] = vec4f(0.0);
        specB[idx] = vec4f(0.0);
        return;
    }
    let w0 = 2.0 * PI_F / S.misc.z;
    let w = floor(omegaOf(kl) / w0) * w0;
    let ph = w * S.timeP.x;
    let e = vec2f(cos(ph), sin(ph));
    let H0 = h0[idx];
    // h0(k) travels along +k: h(k,t) = h0(k) e^{-iwt} + conj(h0(-k)) e^{iwt}
    let h = cmul(H0.xy, vec2f(e.x, -e.y)) + cmul(H0.zw, e);
    let ih = mulI(h);
    let lam = S.misc.x;
    let ik = 1.0 / kl;
    let Dx = ih * (lam * k.x * ik);
    let Dz = ih * (lam * k.y * ik);
    let jxz = h * (-lam * k.x * k.y * ik);
    let sx = ih * k.x;
    let sz = ih * k.y;
    let jxx = h * (-lam * k.x * k.x * ik);
    let jzz = h * (-lam * k.y * k.y * ik);
    specA[idx] = vec4f(Dx + mulI(Dz), h + mulI(jxz));
    specB[idx] = vec4f(sx + mulI(sz), jxx + mulI(jzz));
}

// ---------------------------------------------------------------- inverse FFT

var<workgroup> wa: array<vec4f, 256>;
var<workgroup> wb: array<vec4f, 256>;

fn bitrev(i: u32) -> u32 { return reverseBits(i) >> (32u - LOGN); }

// In-place radix-2 inverse FFT over workgroup memory (input already bit-reversed).
fn butterflies(t: u32) {
    for (var s = 0u; s < LOGN; s++) {
        let halfSize = 1u << s;
        let pos = t & (halfSize - 1u);
        let i0 = ((t >> s) << (s + 1u)) + pos;
        let i1 = i0 + halfSize;
        let ang = PI_F * f32(pos) / f32(halfSize);
        let w = vec2f(cos(ang), sin(ang));
        let a0 = wa[i0];
        let a1 = wa[i1];
        let b0 = wb[i0];
        let b1 = wb[i1];
        let ta = vec4f(cmul(a1.xy, w), cmul(a1.zw, w));
        let tb = vec4f(cmul(b1.xy, w), cmul(b1.zw, w));
        wa[i0] = a0 + ta; wa[i1] = a0 - ta;
        wb[i0] = b0 + tb; wb[i1] = b0 - tb;
        workgroupBarrier();
    }
}

@group(0) @binding(5) var<storage, read> inA: array<vec4f>;
@group(0) @binding(6) var<storage, read> inB: array<vec4f>;
@group(0) @binding(7) var<storage, read_write> outA: array<vec4f>;
@group(0) @binding(8) var<storage, read_write> outB: array<vec4f>;

@compute @workgroup_size(128, 1, 1)
fn ocean_fft_rows(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) t: u32) {
    let base = (wg.z * N + wg.x) * N;
    let e0 = t;
    let e1 = t + N / 2u;
    wa[bitrev(e0)] = inA[base + e0];
    wa[bitrev(e1)] = inA[base + e1];
    wb[bitrev(e0)] = inB[base + e0];
    wb[bitrev(e1)] = inB[base + e1];
    workgroupBarrier();
    butterflies(t);
    outA[base + e0] = wa[e0];
    outA[base + e1] = wa[e1];
    outB[base + e0] = wb[e0];
    outB[base + e1] = wb[e1];
}

@group(0) @binding(9) var displacementOut: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(10) var derivativesOut: texture_storage_2d_array<rgba16float, write>;

@compute @workgroup_size(128, 1, 1)
fn ocean_fft_cols(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) t: u32) {
    let x = wg.x;
    let c = wg.z;
    let e0 = t;
    let e1 = t + N / 2u;
    wa[bitrev(e0)] = inA[(c * N + e0) * N + x];
    wa[bitrev(e1)] = inA[(c * N + e1) * N + x];
    wb[bitrev(e0)] = inB[(c * N + e0) * N + x];
    wb[bitrev(e1)] = inB[(c * N + e1) * N + x];
    workgroupBarrier();
    butterflies(t);
    // a = (Dx, Dz, height, dDx/dz), b = (slope x, slope z, dDx/dx, dDz/dz)
    let a0 = wa[e0];
    let a1 = wa[e1];
    textureStore(displacementOut, vec2u(x, e0), c, vec4f(a0.x, a0.z, a0.y, a0.w));
    textureStore(displacementOut, vec2u(x, e1), c, vec4f(a1.x, a1.z, a1.y, a1.w));
    textureStore(derivativesOut, vec2u(x, e0), c, wb[e0]);
    textureStore(derivativesOut, vec2u(x, e1), c, wb[e1]);
}

// ---------------------------------------------------------------- curvature

@group(0) @binding(11) var derivativesIn: texture_2d_array<f32>;
@group(0) @binding(12) var curvatureOut: texture_storage_2d_array<rgba16float, write>;

// Second derivatives of the height (d2h/dx2, d2h/dxdz, d2h/dz2) from the slopes, for the light focusing.
@compute @workgroup_size(16, 16, 1)
fn ocean_curvature(@builtin(global_invocation_id) id: vec3u) {
    let c = id.z;
    let spacing = S.L[c] / f32(N);
    let xp = (id.x + 1u) % N;
    let xm = (id.x + N - 1u) % N;
    let yp = (id.y + 1u) % N;
    let ym = (id.y + N - 1u) % N;
    let right = textureLoad(derivativesIn, vec2u(xp, id.y), c, 0);
    let left = textureLoad(derivativesIn, vec2u(xm, id.y), c, 0);
    let down = textureLoad(derivativesIn, vec2u(id.x, yp), c, 0);
    let up = textureLoad(derivativesIn, vec2u(id.x, ym), c, 0);
    let hxx = (right.x - left.x) / (2.0 * spacing);
    let hzz = (down.y - up.y) / (2.0 * spacing);
    let hxz = (down.x - up.x) / (2.0 * spacing);
    textureStore(curvatureOut, id.xy, c, vec4f(hxx, hxz, hzz, 0.0));
}

// The curvature gets mipmaps, because the light shafts read a blurred version of it. Each dispatch
// averages 2x2 texels of one level into the next (Metal's `generateMipmaps` does the same).
@group(0) @binding(13) var mipIn: texture_2d_array<f32>;
@group(0) @binding(14) var mipOut: texture_storage_2d_array<rgba16float, write>;

@compute @workgroup_size(8, 8, 1)
fn ocean_curvature_mip(@builtin(global_invocation_id) id: vec3u) {
    let size = textureDimensions(mipOut);
    if (id.x >= size.x || id.y >= size.y) { return; }
    let p = id.xy * 2u;
    let c = id.z;
    let sum = textureLoad(mipIn, p, c, 0) + textureLoad(mipIn, p + vec2u(1u, 0u), c, 0)
            + textureLoad(mipIn, p + vec2u(0u, 1u), c, 0) + textureLoad(mipIn, p + vec2u(1u, 1u), c, 0);
    textureStore(mipOut, id.xy, c, sum * 0.25);
}
