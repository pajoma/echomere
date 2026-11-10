import Foundation

/// The sea surface above every cove: an FFT ocean (Tessendorf) with a wind sea and a long swell, simulated
/// on the GPU (see `OceanShaders` in the renderer). It is the same for every cove and every screen, and it
/// repeats seamlessly every `loopPeriod` seconds, so every screen only needs the time within the loop.
public enum Sea {
    /// Wind speed 10 m above the sea, in m/s.
    public static let windSpeed = 11.0
    /// Direction the wind waves travel, in degrees from the x axis (90 is away from the camera).
    public static let windDirection = 118.0
    /// Distance the wind has blown over open water, in metres.
    public static let fetch = 90_000.0
    public static let jonswapGamma = 3.3
    /// Significant height of the swell, in metres.
    public static let swellHeight = 2.4
    public static let swellDirection = 94.0
    public static let swellWavelength = 120.0
    /// How narrow the swell's directions are (cos^2s exponent): larger is more long-crested.
    public static let swellSpread = 48.0
    /// How much the crests are sharpened by horizontal displacement.
    public static let choppiness = 1.45
    /// Waves shorter than about this length (m) are damped.
    public static let shortWaveCut = 0.0012
    /// Patch sizes of the simulation cascades, in metres: long swell, wind sea, ripples.
    public static let cascadeSizes: [Double] = [211.3, 39.7, 7.43]
    /// The sea repeats after this many seconds; every wave frequency is a multiple of 2π / loopPeriod.
    public static let loopPeriod = 600.0
    /// The seed for the random phases of the spectrum. The same everywhere, so every screen sees the same sea.
    public static let seed: UInt32 = 1

    /// The time within the loop, in seconds, computed in double precision so the GPU gets a small, exact value.
    public static func time(at date: Date) -> Double {
        let t = date.timeIntervalSince1970.truncatingRemainder(dividingBy: loopPeriod)
        return t < 0 ? t + loopPeriod : t
    }
}
