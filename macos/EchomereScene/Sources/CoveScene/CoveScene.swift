import Foundation

public struct CreaturePose: Equatable, Sendable {
    public enum Kind: Equatable, Sendable { case whale, fish }

    public let kind: Kind
    /// The creature ID for whales; a stable name such as "fish-1-4" for ambient life.
    public let id: String
    public let x: Double
    public let y: Double
    /// 0 is near the camera, 1 is far away.
    public let depth: Double
    /// -1 swims left, +1 swims right; values in between are a whale or fish turning.
    public let facing: Double
    /// Tail beat, 0..<2π.
    public let phase: Double
    public let size: Double
    public let opacity: Double
    public let colour: Int
    public let pattern: Int
}

public struct EchoPose: Equatable, Sendable {
    public let x: Double
    public let y: Double
    public let depth: Double
    /// 1 when fresh, fading to 0 over `SceneRules.echoLifetime`.
    public let intensity: Double
    /// Progress 0...1 of a distant call (an expanding ring), or nil when the echo is silent.
    public let call: Double?
    public let size: Double
}

/// Everything on screen at one moment.
public struct SceneFrame: Equatable, Sendable {
    public let time: Date
    public let terrain: [TerrainLayer]
    /// Whales and fish, ordered from far to near.
    public let creatures: [CreaturePose]
    public let kelp: [KelpFrond]
    public let kelpMotion: KelpMotion
    public let echoes: [EchoPose]
    /// The sea surface above the cove at this moment: the time within the sea's loop, in seconds.
    public let seaTime: Double

    public var whales: [CreaturePose] { creatures.filter { $0.kind == .whale } }
    public var fish: [CreaturePose] { creatures.filter { $0.kind == .fish } }
}

/// A cove's scene. Every screen in the cove computes the same frame from the same seed, visitors, echoes and
/// time, so they all show the same picture without anything being streamed (ADR 0005).
public struct CoveScene: Equatable, Sendable {
    public let seed: CoveSeed
    public let layout: CoveLayout

    public init(seed: CoveSeed) {
        self.seed = seed
        self.layout = CoveLayout(seed: seed)
    }

    public func frame(at time: Date, visitors: [Visitor], echoes: [Echo]) -> SceneFrame {
        let t = time.timeIntervalSince1970

        let present = visitors
            .filter { $0.arrivedAt <= time }
            .sorted { ($0.arrivedAt, $0.creatureID) < ($1.arrivedAt, $1.creatureID) }
            .prefix(SceneRules.maxWhales)
        let whales = present.map { whalePose(for: $0, at: t) }

        let fish = layout.schools.enumerated().flatMap { fishPoses(for: $1, schoolIndex: $0, at: t) }

        let creatures = (whales + fish).sorted { ($0.depth, $0.id) > ($1.depth, $1.id) }

        let recentEchoes = echoes
            .filter { $0.leftAt <= time && time.timeIntervalSince($0.leftAt) < SceneRules.echoLifetime }
            .sorted { ($0.leftAt, $0.creatureID) > ($1.leftAt, $1.creatureID) }
            .prefix(SceneRules.maxEchoes)
            .map { echoPose(for: $0, at: t) }

        return SceneFrame(
            time: time, terrain: layout.terrain, creatures: creatures, kelp: layout.kelp,
            kelpMotion: Kelp.motion(at: time), echoes: Array(recentEchoes),
            seaTime: Sea.time(at: time))
    }

    // MARK: - Whales

    /// A whale swims a slow, endless loop through the cove, unique to this cove and this whale. It leaves the
    /// view now and then and comes back.
    struct WhalePath {
        let amplitudeX, periodX, phaseX: Double
        let centreY, amplitudeY, periodY, phaseY: Double
        let depthMid, depthAmplitude, periodDepth, phaseDepth: Double
        let tailPeriod, tailPhase: Double

        init(cove: CoveSeed, creatureID: String) {
            var random = SplitMix64(seed: combine(cove.value, stableHash(creatureID)))
            amplitudeX = random.range(1.05, 1.3)
            periodX = random.range(420, 780)
            phaseX = random.angle()
            centreY = random.range(0.42, 0.68)
            amplitudeY = random.range(0.06, 0.14)
            periodY = random.range(150, 300)
            phaseY = random.angle()
            depthMid = random.range(0.35, 0.65)
            depthAmplitude = random.range(0.1, 0.2)
            periodDepth = random.range(200, 400)
            phaseDepth = random.angle()
            tailPeriod = random.range(5, 8)
            tailPhase = random.angle()
        }

        func position(at t: Double) -> (x: Double, y: Double, depth: Double, velocityX: Double) {
            let ax = cycle(t, periodX) + phaseX
            return (
                SceneRules.worldCentreX + amplitudeX * sin(ax),
                centreY + amplitudeY * sin(cycle(t, periodY) + phaseY),
                depthMid + depthAmplitude * sin(cycle(t, periodDepth) + phaseDepth),
                amplitudeX * (2 * .pi / periodX) * cos(ax))
        }
    }

    func whalePose(for visitor: Visitor, at t: Double) -> CreaturePose {
        let path = WhalePath(cove: seed, creatureID: visitor.creatureID)
        let position = path.position(at: t)
        let sinceArrival = t - visitor.arrivedAt.timeIntervalSince1970
        return CreaturePose(
            kind: .whale,
            id: visitor.creatureID,
            x: position.x,
            y: position.y,
            depth: position.depth,
            facing: tanh(position.velocityX / 0.004),
            phase: wrapAngle(cycle(t, path.tailPeriod) + path.tailPhase),
            size: visitor.look.size,
            opacity: min(1, max(0, sinceArrival / SceneRules.whaleFadeIn)),
            colour: visitor.look.colour,
            pattern: visitor.look.pattern)
    }

    // MARK: - Ambient life

    func fishPoses(for school: FishSchool, schoolIndex: Int, at t: Double) -> [CreaturePose] {
        let ax = cycle(t, school.periodX) + school.phaseX
        let centreX = school.centreX + school.amplitudeX * sin(ax)
        let centreY = school.centreY + school.amplitudeY * sin(cycle(t, school.periodY) + school.phaseY)
        let velocityX = school.amplitudeX * (2 * .pi / school.periodX) * cos(ax)
        let facing = tanh(velocityX / 0.002)
        return school.members.enumerated().map { index, member in
            CreaturePose(
                kind: .fish,
                id: "fish-\(schoolIndex)-\(index)",
                x: centreX + member.offsetX + 0.008 * sin(t * 0.6 + member.wobblePhase),
                y: centreY + member.offsetY + 0.006 * sin(t * 0.9 + member.wobblePhase * 1.3),
                depth: school.depth,
                facing: facing,
                phase: wrapAngle(t * 2 * .pi * 1.6 + member.wobblePhase),
                size: school.size * member.size,
                opacity: 1,
                colour: school.tint,
                pattern: 0)
        }
    }

    // MARK: - Echoes

    func echoPose(for echo: Echo, at t: Double) -> EchoPose {
        let left = echo.leftAt.timeIntervalSince1970
        // The echo stays where the whale swam in this cove when it left.
        let position = WhalePath(cove: seed, creatureID: echo.creatureID).position(at: left)
        var random = SplitMix64(seed: combine(seed.value ^ 0xEC40, stableHash(echo.creatureID)))
        let callPeriod = random.range(900, 2400)
        let callOffset = random.range(0, callPeriod)
        let callDuration = 8.0
        let sinceCall = (t - left + callOffset).truncatingRemainder(dividingBy: callPeriod)
        let width = SceneRules.worldWidth
        return EchoPose(
            x: min(max(position.x, 0.1 * width), 0.9 * width),
            y: position.y,
            depth: position.depth,
            intensity: max(0, 1 - (t - left) / SceneRules.echoLifetime),
            call: sinceCall < callDuration ? sinceCall / callDuration : nil,
            size: echo.look.size)
    }
}

/// The angle 0..<2π that `t` has reached in a cycle of `period` seconds. Reducing first keeps it precise for
/// large timestamps.
func cycle(_ t: Double, _ period: Double) -> Double {
    (t / period).truncatingRemainder(dividingBy: 1) * 2 * .pi
}

func wrapAngle(_ angle: Double) -> Double {
    let wrapped = angle.truncatingRemainder(dividingBy: 2 * .pi)
    return wrapped < 0 ? wrapped + 2 * .pi : wrapped
}
