//! A small, portable random generator (SplitMix64), so that every screen derives the same scene from the
//! same seed. The scene must never use a system random source.

/// Exact port of `Random.swift`'s `SplitMix64`, with wrapping arithmetic.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in 0..<1.
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    pub fn range(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.unit()
    }

    /// A value in low..=high.
    pub fn int(&mut self, low: i64, high: i64) -> i64 {
        let span = (high - low + 1) as u64;
        low + (self.next() % span) as i64
    }

    pub fn angle(&mut self) -> f64 {
        self.range(0.0, 2.0 * std::f64::consts::PI)
    }
}

/// FNV-1a: a hash that is the same on every machine and every run (unlike `std::hash`).
pub fn stable_hash(text: &str) -> u64 {
    let mut hash: u64 = 0xCBF2_9CE4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

pub fn combine(a: u64, b: u64) -> u64 {
    let mut generator = SplitMix64::new(a ^ b.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    generator.next()
}
