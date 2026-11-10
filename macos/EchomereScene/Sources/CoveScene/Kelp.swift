import Foundation

/// One stalk (stipe) of a giant kelp plant, in metres. x is to the right of the camera, z is ahead of it.
/// It is rooted on the seabed below the view, rises to the surface, and trails along it in the current.
public struct KelpFrond: Equatable, Sendable {
    public let baseX: Double
    public let baseZ: Double
    /// How far below the surface the stalk is rooted.
    public let baseDepth: Double
    /// Length of the stalk: its depth plus the part that trails along the surface.
    public let length: Double
    /// Where this stalk is in its sway, relative to the swell (radians).
    public let swayPhase: Double
    /// How the stalks of one plant wind around each other (radians).
    public let twistPhase: Double
    /// Varies the blades from stalk to stalk.
    public let bladeSeed: Double
    public let bladeScale: Double
}

/// Where the kelp is in its movement at one moment, as angles 0..<2π.
public struct KelpMotion: Equatable, Sendable {
    /// The back-and-forth surge of the swell.
    public let surge: Double
    /// A slow sideways drift.
    public let drift: Double
    /// The flutter of the blades.
    public let flutter: Double
}

/// How kelp moves (see the kelp research in PR #10: linear wave theory for the surge).
public enum Kelp {
    public static let driftPeriod = 60.0
    public static let flutterPeriod = 1.6

    /// The period of the swell over the cove's seabed, in seconds: ω² = g·k·tanh(k·h).
    public static var surgePeriod: Double {
        let k = 2 * Double.pi / Sea.swellWavelength
        let omega = (9.81 * k * tanh(k * SceneRules.cameraDepth)).squareRoot()
        return 2 * .pi / omega
    }

    public static func motion(at time: Date) -> KelpMotion {
        let t = time.timeIntervalSince1970
        return KelpMotion(surge: cycle(t, surgePeriod), drift: cycle(t, driftPeriod), flutter: cycle(t, flutterPeriod))
    }
}
