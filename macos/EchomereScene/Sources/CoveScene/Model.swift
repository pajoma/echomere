import Foundation

/// The seed a cove's scene is generated from. It comes from the backend and never changes.
public struct CoveSeed: Hashable, Sendable {
    public let value: UInt64
    public init(_ value: UInt64) { self.value = value }
}

/// The kinds of whale a creator can choose from.
public enum WhaleSpecies: Int, CaseIterable, Hashable, Sendable {
    case southernRightWhale
    case orca

    /// Length of an average adult, in metres.
    public var length: Double {
        switch self {
        case .southernRightWhale: 14.5
        case .orca: 7.0
        }
    }
}

/// How a whale looks, chosen by its creator from fixed options.
public struct WhaleLook: Hashable, Sendable {
    public static let sizeRange = 0.8...1.2

    public let species: WhaleSpecies
    /// Relative to an average adult of its species.
    public let size: Double

    public init(species: WhaleSpecies, size: Double) {
        self.species = species
        self.size = min(max(size, Self.sizeRange.lowerBound), Self.sizeRange.upperBound)
    }

    /// Length in metres.
    public var length: Double { species.length * size }
}

/// A creature visiting the cove during its current stop.
public struct Visitor: Hashable, Sendable {
    public let creatureID: String
    public let look: WhaleLook
    public let arrivedAt: Date

    public init(creatureID: String, look: WhaleLook, arrivedAt: Date) {
        self.creatureID = creatureID
        self.look = look
        self.arrivedAt = arrivedAt
    }
}

/// The trace a creature left in the cove when it moved on.
public struct Echo: Hashable, Sendable {
    public let creatureID: String
    public let look: WhaleLook
    public let leftAt: Date

    public init(creatureID: String, look: WhaleLook, leftAt: Date) {
        self.creatureID = creatureID
        self.look = look
        self.leftAt = leftAt
    }
}

/// The rules the scene follows (see the version 1 spec, #2, and ADR 0005).
public enum SceneRules {
    /// Five visitors, plus the creator's own new whale on its first day.
    public static let maxWhales = 6
    public static let maxEchoes = 5
    public static let echoLifetime: TimeInterval = 7 * 24 * 3600
    public static let whaleFadeIn: TimeInterval = 120
    /// The scene is 1.6 units wide and 1 unit high. Screens with another aspect ratio see more or less of
    /// the sea to the left and right, always centred on `worldCentreX`.
    public static let worldWidth = 1.6
    public static let worldCentreX = 0.8
    /// The camera stands on the seabed this far below the surface, in metres. Must match `CAMERA_DEPTH` in the
    /// shader.
    public static let cameraDepth = 30.0
    /// The camera is tilted up by this angle (radians) and has this focal length, in picture heights. Must match
    /// `PITCH` and `FOCAL` in the shader.
    public static let cameraPitch = 0.262
    public static let cameraFocal = 0.714
}
