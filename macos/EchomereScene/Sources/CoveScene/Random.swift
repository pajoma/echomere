/// A small, portable random generator (SplitMix64), so that every screen derives the same scene from the
/// same seed. `SystemRandomNumberGenerator` must never be used in the scene.
struct SplitMix64 {
    private var state: UInt64

    init(seed: UInt64) { state = seed }

    mutating func next() -> UInt64 {
        state &+= 0x9E37_79B9_7F4A_7C15
        var z = state
        z = (z ^ (z >> 30)) &* 0xBF58_476D_1CE4_E5B9
        z = (z ^ (z >> 27)) &* 0x94D0_49BB_1331_11EB
        return z ^ (z >> 31)
    }

    /// A value in 0..<1.
    mutating func unit() -> Double { Double(next() >> 11) * 0x1.0p-53 }

    mutating func range(_ low: Double, _ high: Double) -> Double { low + (high - low) * unit() }

    /// A value in low...high.
    mutating func int(_ low: Int, _ high: Int) -> Int { low + Int(next() % UInt64(high - low + 1)) }

    mutating func angle() -> Double { range(0, 2 * .pi) }
}

/// FNV-1a: a hash that is the same on every machine and every run (unlike `Hasher`).
func stableHash(_ text: String) -> UInt64 {
    var hash: UInt64 = 0xCBF2_9CE4_8422_2325
    for byte in text.utf8 {
        hash ^= UInt64(byte)
        hash = hash &* 0x0000_0100_0000_01B3
    }
    return hash
}

func combine(_ a: UInt64, _ b: UInt64) -> UInt64 {
    var generator = SplitMix64(seed: a ^ (b &* 0x9E37_79B9_7F4A_7C15))
    return generator.next()
}
