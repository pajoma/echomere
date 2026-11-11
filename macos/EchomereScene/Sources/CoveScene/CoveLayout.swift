import Foundation

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

/// Everything about a cove that is generated once from its seed: kelp and ambient life.
public struct CoveLayout: Equatable, Sendable {
    public let kelp: [KelpFrond]
    public let schools: [FishSchool]

    public init(seed: CoveSeed) {
        var random = SplitMix64(seed: seed.value)

        // Giant kelp, only hinted at in the background: a few plants far from the camera, faint in the haze. They
        // are rooted where giant kelp really grows, 25 to 30 m down, on ground that rises gently in the distance;
        // their roots are lost in the gloom.
        var fronds: [KelpFrond] = []
        let plants = random.int(5, 7)
        for plant in 0..<plants {
            let z = 55 + 45 * (Double(plant) + random.unit()) / Double(plants)
            let x = random.range(-0.95, 0.95) * z
            let depth = SceneRules.cameraDepth - 0.1 * (z - 55)
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

        schools = (0..<random.int(2, 3)).map { _ in
            FishSchool(
                // The small fish stay near the camera, in front of the whales and the kelp.
                depth: random.range(0.05, 0.35),
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
                members: (0..<random.int(20, 34)).map { _ in
                    FishSchool.Member(
                        offsetX: random.range(-0.10, 0.10),
                        offsetY: random.range(-0.045, 0.045),
                        wobblePhase: random.angle(),
                        size: random.range(0.85, 1.15))
                })
        }
    }
}
