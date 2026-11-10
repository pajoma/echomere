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
    public let kelp: [KelpFrond]
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

        // Giant kelp: a few plants, each a bundle of stalks. They are rooted on the seabed, which slopes away
        // below the view, so the stalks rise into the picture from its bottom edge.
        var fronds: [KelpFrond] = []
        let plants = random.int(9, 12)
        for plant in 0..<plants {
            // Plants stand at many distances: the near ones are dark shapes, the far ones fade into the water.
            let z = 6 + 54 * pow((Double(plant) + random.unit()) / Double(plants), 1.4)
            // Near plants stand to the left and right, leaving a clearing ahead of the camera for the whales;
            // far ones stand anywhere.
            let side = plant % 2 == 0 ? -1.0 : 1.0
            let x = z < 28 ? side * random.range(0.38, 1.0) * z : random.range(-0.9, 0.9) * z
            let depth = SceneRules.cameraDepth + 2 + 0.36 * z
            let plantPhase = random.angle()
            for _ in 0..<random.int(2, 4) {
                fronds.append(KelpFrond(
                    baseX: x + random.range(-0.12, 0.12),
                    baseZ: z + random.range(-0.12, 0.12),
                    baseDepth: depth,
                    length: depth + random.range(2, 7),
                    swayPhase: plantPhase + random.range(-0.25, 0.25),
                    twistPhase: random.angle(),
                    bladeSeed: random.range(0, 100),
                    bladeScale: random.range(0.85, 1.2)))
            }
        }
        kelp = fronds

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
