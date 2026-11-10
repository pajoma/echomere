import Foundation

/// One ridge of the seabed. Height is a sum of four sine waves over x.
/// The renderer's shader evaluates the same formula (`terrainHeight`); keep both in step.
public struct TerrainLayer: Equatable, Sendable {
    /// 0 is near the camera, 1 is far away.
    public let depth: Double
    public let base: Double
    public let amplitudes: SIMD4<Double>
    public let frequencies: SIMD4<Double>
    public let phases: SIMD4<Double>

    public func height(at x: Double) -> Double {
        var height = base
        for k in 0..<4 {
            height += amplitudes[k] * sin(frequencies[k] * x + phases[k])
        }
        return height
    }
}

public struct KelpStalk: Equatable, Sendable {
    public let rootX: Double
    public let rootY: Double
    public let height: Double
    public let depth: Double
    public let swayAmplitude: Double
    public let swayPeriod: Double
    public let phase: Double
    public let width: Double
}

public struct FishSchool: Equatable, Sendable {
    public struct Member: Equatable, Sendable {
        public let offsetX: Double
        public let offsetY: Double
        public let wobblePhase: Double
        public let size: Double
    }

    public let depth: Double
    public let size: Double
    /// 0 is silver, 3 is golden.
    public let tint: Int
    public let centreX: Double
    public let centreY: Double
    public let amplitudeX: Double
    public let amplitudeY: Double
    public let periodX: Double
    public let periodY: Double
    public let phaseX: Double
    public let phaseY: Double
    public let members: [Member]
}

/// Everything about a cove that is generated once from its seed: seabed, kelp and ambient life.
public struct CoveLayout: Equatable, Sendable {
    public let terrain: [TerrainLayer]
    public let kelp: [KelpStalk]
    public let schools: [FishSchool]

    public init(seed: CoveSeed) {
        var random = SplitMix64(seed: seed.value)

        // Far, middle and near ridges.
        let ridges: [(depth: Double, base: Double, amplitude: Double)] = [
            (0.9, 0.30, 0.08), (0.6, 0.19, 0.06), (0.25, 0.08, 0.04),
        ]
        terrain = ridges.map { ridge in
            TerrainLayer(
                depth: ridge.depth,
                base: ridge.base + random.range(-0.03, 0.03),
                amplitudes: SIMD4(
                    ridge.amplitude * random.range(0.6, 1.0),
                    ridge.amplitude * 0.5 * random.range(0.5, 1.0),
                    ridge.amplitude * 0.25 * random.range(0.5, 1.0),
                    ridge.amplitude * 0.12 * random.range(0.5, 1.0)),
                frequencies: SIMD4(
                    random.range(1.5, 2.5), random.range(4, 6), random.range(9, 13), random.range(20, 28)),
                phases: SIMD4(random.angle(), random.angle(), random.angle(), random.angle()))
        }

        // Kelp grows on the near ridge.
        let seabed = terrain[2]
        var stalks: [KelpStalk] = []
        for _ in 0..<random.int(1, 3) {
            let standX = random.range(0.05, 1.55)
            for _ in 0..<random.int(4, 9) {
                let rootX = standX + random.range(-0.07, 0.07)
                stalks.append(KelpStalk(
                    rootX: rootX,
                    rootY: seabed.height(at: rootX) - 0.01,
                    height: random.range(0.25, 0.62),
                    depth: random.range(0.15, 0.45),
                    swayAmplitude: random.range(0.02, 0.05),
                    swayPeriod: random.range(6, 10),
                    phase: random.angle(),
                    width: random.range(0.003, 0.006)))
            }
        }
        kelp = stalks

        schools = (0..<random.int(2, 4)).map { _ in
            FishSchool(
                depth: random.range(0.3, 0.85),
                size: random.range(0.8, 1.3),
                tint: random.int(0, 3),
                centreX: random.range(0.3, 1.3),
                centreY: random.range(0.38, 0.75),
                amplitudeX: random.range(0.3, 0.7),
                amplitudeY: random.range(0.04, 0.1),
                periodX: random.range(70, 200),
                periodY: random.range(30, 80),
                phaseX: random.angle(),
                phaseY: random.angle(),
                members: (0..<random.int(7, 14)).map { _ in
                    FishSchool.Member(
                        offsetX: random.range(-0.05, 0.05),
                        offsetY: random.range(-0.025, 0.025),
                        wobblePhase: random.angle(),
                        size: random.range(0.85, 1.15))
                })
        }
    }
}
