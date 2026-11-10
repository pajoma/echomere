import Foundation

/// One travelling wave on the sea surface: height = amplitude · cos(kx·x + kz·z − phase), in metres.
public struct Wave: Equatable, Sendable {
    public let kx: Double
    public let kz: Double
    public let amplitude: Double
    /// The wave's phase at this moment, 0..<2π.
    public let phase: Double

    public var wavenumber: Double { (kx * kx + kz * kz).squareRoot() }
}

/// The sea surface above every cove: a sum of travelling waves that obey the deep-water dispersion relation
/// ω = √(g·k), so long waves travel faster than short ones, as at sea. It is the same for every cove and every
/// screen. The surface, the light shafts and the caustics on the seabed are all drawn from it.
public enum Sea {
    public static let gravity = 9.81
    /// The wave field repeats every `patchSize` metres in both directions, so the renderer can tile it.
    /// Must match `PATCH` in the shader.
    public static let patchSize = 32.0
    public static let maxWaves = 24
    /// The first `swellWaves` waves shape the light shafts and caustics; the shorter ones after them only
    /// add the fine shimmer on the surface. Must match `SWELL_WAVES` in the shader.
    public static let swellWaves = 14

    /// Wave numbers on the patch grid (so the field tiles seamlessly), steepness k·A, and a starting phase.
    /// The wind blows mostly along x; the waves spread to both sides of it.
    static let components: [(m: Int, n: Int, steepness: Double, phase: Double)] = [
        (2, 1, 0.050, 0.3), (3, -1, 0.050, 1.7), (4, 2, 0.048, 4.1), (5, -2, 0.046, 2.2),
        (7, 1, 0.044, 5.5), (8, -3, 0.042, 0.9), (10, 2, 0.040, 3.3), (12, -4, 0.038, 6.0),
        (15, 3, 0.034, 1.2), (18, -2, 0.032, 4.7), (22, 5, 0.028, 2.8), (26, -6, 0.026, 0.5),
        (31, 4, 0.022, 3.9), (37, -3, 0.020, 5.1),
        // Short waves, 0.5 to 0.8 m: the shimmer.
        (41, 8, 0.030, 2.4), (47, -6, 0.030, 0.6), (53, 11, 0.028, 4.4), (59, -9, 0.028, 1.9),
        (36, 14, 0.026, 5.8), (64, 4, 0.026, 3.1), (29, -15, 0.024, 0.2), (70, -12, 0.024, 4.9),
    ]

    public static func angularFrequency(wavenumber k: Double) -> Double { (gravity * k).squareRoot() }

    /// The waves at a moment. Phases are reduced in double precision, so the shader gets small, exact angles
    /// even years after 1970.
    public static func waves(at time: Date) -> [Wave] {
        let t = time.timeIntervalSince1970
        let step = 2 * Double.pi / patchSize
        return components.map { c in
            let kx = Double(c.m) * step
            let kz = Double(c.n) * step
            let k = (kx * kx + kz * kz).squareRoot()
            let omega = angularFrequency(wavenumber: k)
            let cycles = (omega * t / (2 * .pi)).truncatingRemainder(dividingBy: 1)
            return Wave(kx: kx, kz: kz, amplitude: c.steepness / k,
                        phase: wrapAngle(cycles * 2 * .pi + c.phase))
        }
    }
}
