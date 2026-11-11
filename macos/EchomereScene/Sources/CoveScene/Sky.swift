import Foundation

/// The sky above the cove at one moment: where the light comes from, how strong and what colour it is, and
/// the colours of the sky itself. The sun is up from 6 to 18 o'clock and the moon from 18 to 6.
public struct SkyState: Equatable, Sendable {
    /// The hour of the day, 0..<24.
    public let hour: Double
    /// Direction towards the sun by day, towards the moon by night (x right, y up, z ahead of the camera).
    public let lightDirection: SIMD3<Double>
    /// 1 for the sun high in the sky, about 0.5 for the full moon, 0 while neither is above the horizon.
    public let lightIntensity: Double
    public let lightColour: SIMD3<Double>
    public let zenith: SIMD3<Double>
    public let horizon: SIMD3<Double>
    /// How bright the water itself is: 1 at the golden hour, more at noon, little at night.
    public let ambient: Double
    public let isNight: Bool
}

public enum Sky {
    public static let sunPeakElevation = 60.0
    public static let moonPeakElevation = 45.0

    /// The hour of the day at a moment. For now this is UTC for every cove.
    public static func hour(at date: Date) -> Double {
        let h = (date.timeIntervalSince1970 / 3600).truncatingRemainder(dividingBy: 24)
        return h < 0 ? h + 24 : h
    }

    public static func state(at date: Date) -> SkyState { state(hour: hour(at: date)) }

    /// Elevation of the sun in degrees: 0 at 6 and 18 o'clock, highest at noon, below the horizon at night.
    public static func sunElevation(hour: Double) -> Double {
        sunPeakElevation * sin(.pi * (hour - 6) / 12)
    }

    public static func moonElevation(hour: Double) -> Double {
        moonPeakElevation * sin(.pi * (hour - 18) / 12)
    }

    public static func state(hour rawHour: Double) -> SkyState {
        let hour = ((rawHour.truncatingRemainder(dividingBy: 24)) + 24).truncatingRemainder(dividingBy: 24)
        let sun = sunElevation(hour: hour)
        let moon = moonElevation(hour: hour)
        let night = sun < 0

        // Both rise ahead of the camera to its right, and set ahead of it to its left, so the camera sees
        // every sunrise, sunset, moonrise and moonset.
        let sinceRise = night ? (hour < 6 ? hour + 6 : hour - 18) : hour - 6
        let azimuth = (0.5 - sinceRise / 12) * 80 * .pi / 180
        let elevation = max(night ? moon : sun, 0) * .pi / 180
        let direction = SIMD3(sin(azimuth) * cos(elevation), sin(elevation), cos(azimuth) * cos(elevation))

        let sunStrength = smooth(0, 10, sun) * (0.6 + 0.4 * smooth(10, 40, sun))
        let moonStrength = 0.5 * smooth(0, 10, moon)
        let moonlight = SIMD3(0.70, 0.80, 1.00)

        // Colours by the sun's elevation: night, twilight, sunrise or sunset, golden hour, day.
        let keys: [Double] = [-12, -4, 2, 10, 32]
        let zenith = interpolate(sun, keys, [
            SIMD3(0.016, 0.026, 0.060), SIMD3(0.10, 0.12, 0.26), SIMD3(0.38, 0.32, 0.40),
            SIMD3(0.48, 0.41, 0.42), SIMD3(0.20, 0.42, 0.85),
        ]) + moonlight * moonStrength * 0.22
        let horizon = interpolate(sun, keys, [
            SIMD3(0.030, 0.045, 0.090), SIMD3(0.55, 0.30, 0.28), SIMD3(1.00, 0.55, 0.25),
            SIMD3(1.00, 0.64, 0.30), SIMD3(0.62, 0.76, 0.95),
        ]) + moonlight * moonStrength * 0.22
        let sunColour = interpolate(sun, keys, [
            SIMD3(1.00, 0.50, 0.20), SIMD3(1.00, 0.50, 0.20), SIMD3(1.00, 0.55, 0.22),
            SIMD3(1.00, 0.68, 0.32), SIMD3(1.00, 0.96, 0.88),
        ])
        let ambient = interpolate(sun, keys, [
            SIMD3(repeating: 0.26), SIMD3(repeating: 0.45), SIMD3(repeating: 0.75),
            SIMD3(repeating: 1.00), SIMD3(repeating: 1.45),
        ]).x + moonStrength * 0.45

        return SkyState(
            hour: hour, lightDirection: direction, lightIntensity: night ? moonStrength : sunStrength,
            lightColour: night ? moonlight : sunColour, zenith: zenith, horizon: horizon, ambient: ambient,
            isNight: night)
    }

    static func smooth(_ low: Double, _ high: Double, _ x: Double) -> Double {
        let t = min(max((x - low) / (high - low), 0), 1)
        return t * t * (3 - 2 * t)
    }

    static func interpolate(_ x: Double, _ keys: [Double], _ values: [SIMD3<Double>]) -> SIMD3<Double> {
        if x <= keys[0] { return values[0] }
        for i in 1..<keys.count where x <= keys[i] {
            let t = (x - keys[i - 1]) / (keys[i] - keys[i - 1])
            return values[i - 1] + (values[i] - values[i - 1]) * t
        }
        return values[values.count - 1]
    }
}
