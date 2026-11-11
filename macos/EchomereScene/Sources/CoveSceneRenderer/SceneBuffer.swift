import CoveScene
import Foundation

/// Packs a scene frame into the flat list of float4 values the shader reads.
/// The layout must match the constants at the top of `SceneShaders.source`.
public enum SceneBuffer {
    /// The sky: light direction and intensity, light colour and ambient, zenith, horizon.
    static let skyStart = 2
    static let creatureStart = 14
    static let echoStart = 326
    static let count = 336

    static let maxCreatures = (echoStart - creatureStart) / 3
    static let maxEchoes = (count - echoStart) / 2

    /// - Parameters:
    ///   - aspect: width / height of the target.
    ///   - pixelsHigh: height of the target in pixels, for anti-aliasing.
    public static func pack(_ frame: SceneFrame, aspect: Double, pixelsHigh: Double,
                            seaFromAbove: Bool = false) -> [SIMD4<Float>] {
        var data = [SIMD4<Float>](repeating: .zero, count: count)
        let creatures = Array(frame.creatures.prefix(maxCreatures))
        let echoes = Array(frame.echoes.prefix(maxEchoes))

        // Shader time runs over one day; Float keeps enough precision for slow motion at that range.
        let shaderTime = frame.time.timeIntervalSince1970.truncatingRemainder(dividingBy: 86_400)
        data[0] = SIMD4(Float(aspect), Float(shaderTime), Float(creatures.count), 0)
        data[1] = SIMD4(Float(echoes.count), Float(1 / pixelsHigh), seaFromAbove ? 1 : 0, 0)

        for (i, c) in creatures.enumerated() {
            let b = creatureStart + i * 3
            data[b] = SIMD4(Float(c.x), Float(c.y), Float(c.depth), Float(c.facing))
            data[b + 1] = SIMD4(Float(c.phase), Float(c.size), 1, 0)
            data[b + 2] = SIMD4(Float(c.colour), 1, 0, 0)
        }

        let sky = frame.sky
        data[skyStart] = SIMD4(SIMD3<Float>(sky.lightDirection), Float(sky.lightIntensity))
        data[skyStart + 1] = SIMD4(SIMD3<Float>(sky.lightColour), Float(sky.ambient))
        data[skyStart + 2] = SIMD4(SIMD3<Float>(sky.zenith), 0)
        data[skyStart + 3] = SIMD4(SIMD3<Float>(sky.horizon), 0)

        for (i, e) in echoes.enumerated() {
            let b = echoStart + i * 2
            data[b] = SIMD4(Float(e.x), Float(e.y), Float(e.depth), Float(e.intensity))
            data[b + 1] = SIMD4(Float(e.call ?? -1), Float(e.size), 0, 0)
        }
        return data
    }
}
