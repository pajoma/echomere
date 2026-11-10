import Foundation

/// One of the small fish of the ambient life, in picture coordinates.
public struct CreaturePose: Equatable, Sendable {
    /// A stable name such as "fish-1-4".
    public let id: String
    public let x: Double
    public let y: Double
    /// 0 is near the camera, 1 is far away.
    public let depth: Double
    /// -1 swims left, +1 swims right; values in between are a fish turning.
    public let facing: Double
    /// Tail beat, 0..<2π.
    public let phase: Double
    public let size: Double
    public let colour: Int
}

/// A whale in the water, in metres: x to the right of the camera, y up (0 is the surface), z ahead of it.
public struct WhalePose: Equatable, Sendable {
    /// The creature ID.
    public let id: String
    public let species: WhaleSpecies
    public let position: SIMD3<Double>
    /// The way the whale swims (a unit vector).
    public let forward: SIMD3<Double>
    /// How far it banks into a turn, in radians.
    public let roll: Double
    /// Tail beat, 0..<2π.
    public let tailPhase: Double
    /// Length in metres.
    public let length: Double
    /// How hard it swims: 1 beats the tail fully, near 0 it glides.
    public let effort: Double
    public let opacity: Double
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
    /// The whales visiting the cove.
    public let whales: [WhalePose]
    /// The small fish of the ambient life, ordered from far to near.
    public let creatures: [CreaturePose]
    public let kelp: [KelpFrond]
    public let kelpMotion: KelpMotion
    public let echoes: [EchoPose]
    /// The sea surface above the cove at this moment: the time within the sea's loop, in seconds.
    public let seaTime: Double
    /// The sky above the cove. It can be replaced to look at another time of day.
    public var sky: SkyState

    public var fish: [CreaturePose] { creatures }
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

        let creatures = fish.sorted { ($0.depth, $0.id) > ($1.depth, $1.id) }

        let recentEchoes = echoes
            .filter { $0.leftAt <= time && time.timeIntervalSince($0.leftAt) < SceneRules.echoLifetime }
            .sorted { ($0.leftAt, $0.creatureID) > ($1.leftAt, $1.creatureID) }
            .prefix(SceneRules.maxEchoes)
            .map { echoPose(for: $0, at: t) }

        return SceneFrame(
            time: time, terrain: layout.terrain, whales: whales, creatures: creatures, kelp: layout.kelp,
            kelpMotion: Kelp.motion(at: time), echoes: Array(recentEchoes),
            seaTime: Sea.time(at: time), sky: Sky.state(at: time))
    }

    // MARK: - Whales

    /// A whale roams the cove on a slow, endless path, unique to this cove and this whale. It swims at about
    /// walking pace, mostly far from the camera. Only rarely, on a slow cycle of its own, does it come to the
    /// front. It dives, each time to a different depth, and comes up to the surface to take a series of breaths
    /// before it dives again.
    struct WhalePath {
        let amplitudeX, periodX, phaseX: Double
        let centreZ, amplitudeZ, periodZ, phaseZ: Double
        let centreY, amplitudeY, periodY, phaseY: Double
        let approachPeriod, approachPhase, nearest: Double
        let tailPeriod, tailPhase: Double
        let seed: UInt64
        let breathingCycle, breathingOffset, breathInterval: Double
        let breaths: Int
        /// Depth of the whale's middle when its back breaks the surface, in metres.
        let surfaceDepth: Double
        let rollPeriod, rollPhase: Double

        init(cove: CoveSeed, creatureID: String, look: WhaleLook) {
            seed = combine(cove.value, stableHash(creatureID))
            var random = SplitMix64(seed: seed)
            amplitudeX = random.range(95, 125)
            periodX = random.range(520, 760)
            phaseX = random.angle()
            centreZ = random.range(62, 76)
            amplitudeZ = random.range(14, 20)
            periodZ = random.range(400, 640)
            phaseZ = random.angle()
            centreY = random.range(-17, -13)
            amplitudeY = random.range(5, 8)
            periodY = random.range(240, 420)
            phaseY = random.angle()
            approachPeriod = random.range(3600, 7200)
            approachPhase = random.angle()
            nearest = random.range(14, 22)
            tailPeriod = random.range(4.5, 6.5)
            tailPhase = random.angle()
            let species = look.species
            breathingCycle = random.range(species.breathingCycle.lowerBound, species.breathingCycle.upperBound)
            breathingOffset = random.range(0, breathingCycle)
            breaths = random.int(species.breaths.lowerBound, species.breaths.upperBound)
            breathInterval = species.breathInterval * random.range(0.85, 1.15)
            surfaceDepth = 0.085 * look.length
            rollPeriod = random.range(70, 130)
            rollPhase = random.angle()
        }

        /// How close the whale is to its breathing at the surface (1) or to its dive (0), with the depth it
        /// keeps in each.
        func breathing(at t: Double) -> (atSurface: Double, surfaceY: Double, diveY: Double) {
            let cycles = (t + breathingOffset) / breathingCycle
            let phase = cycles - cycles.rounded(.down)
            let signed = phase > 0.5 ? phase - 1 : phase            // the surface series is centred on 0
            let half = Double(breaths) * breathInterval / 2 / breathingCycle
            let ramp = 45 / breathingCycle                           // going up or down takes about 45 s
            let atSurface = 1 - Sky.smooth(half, half + ramp, abs(signed))

            // Between breaths the whale dips a little; at each breath its back breaks the surface.
            let u = min(max((signed + half) / (2 * half), 0), 1)
            let dip = 0.5 + 0.5 * cos(2 * Double.pi * Double(breaths) * u)
            let surfaceY = -surfaceDepth * (1 + 2.2 * dip)

            // Each dive goes to its own depth. The dive changes while the whale is at the surface.
            let dive = UInt64(max((cycles + 0.5).rounded(.down), 0))
            var random = SplitMix64(seed: combine(seed, dive))
            let diveCentre = random.range(-22, -10)
            let diveY = diveCentre + 3.5 * sin(cycle(t, periodY) + phaseY)
            return (atSurface, surfaceY, diveY)
        }

        func position(at t: Double) -> SIMD3<Double> {
            // The visit to the front: 0 for most of the cycle, rising to 1 for a short while.
            let visit = pow(0.5 + 0.5 * sin(cycle(t, approachPeriod) + approachPhase), 90)
            let far = centreZ + amplitudeZ * sin(cycle(t, periodZ) + phaseZ)
            let breath = breathing(at: t)
            return SIMD3(
                amplitudeX * (1 - 0.8 * visit) * sin(cycle(t, periodX) + phaseX),
                breath.diveY + (breath.surfaceY - breath.diveY) * breath.atSurface,
                far + (nearest - far) * visit)
        }

        struct State {
            let position: SIMD3<Double>
            let velocity: SIMD3<Double>
            let acceleration: SIMD3<Double>
        }

        func state(at t: Double) -> State {
            let step = 0.5
            let before = position(at: t - step), now = position(at: t), after = position(at: t + step)
            return State(position: now, velocity: (after - before) / (2 * step),
                         acceleration: (after - 2 * now + before) / (step * step))
        }
    }

    func whalePose(for visitor: Visitor, at t: Double) -> WhalePose {
        let path = WhalePath(cove: seed, creatureID: visitor.creatureID, look: visitor.look)
        let state = path.state(at: t)
        let v = state.velocity, a = state.acceleration
        let speed = max((v * v).sum().squareRoot(), 1e-6)
        // The whale banks into its turns: the faster it turns, the further it leans.
        let turnRate = (v.z * a.x - v.x * a.z) / max(v.x * v.x + v.z * v.z, 1e-6)
        let sinceArrival = t - visitor.arrivedAt.timeIntervalSince1970
        return WhalePose(
            id: visitor.creatureID,
            species: visitor.look.species,
            position: state.position,
            forward: v / speed,
            // It also rolls a little, lazily, as it swims.
            roll: min(max(turnRate * 8, -0.35), 0.35) + 0.12 * sin(cycle(t, path.rollPeriod) + path.rollPhase),
            tailPhase: wrapAngle(cycle(t, path.tailPeriod) + path.tailPhase),
            length: visitor.look.length,
            effort: min(max(speed / 0.8, 0.15), 1),
            opacity: min(1, max(0, sinceArrival / SceneRules.whaleFadeIn)))
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
                id: "fish-\(schoolIndex)-\(index)",
                x: centreX + member.offsetX + 0.008 * sin(t * 0.6 + member.wobblePhase),
                y: centreY + member.offsetY + 0.006 * sin(t * 0.9 + member.wobblePhase * 1.3),
                depth: school.depth,
                facing: facing,
                phase: wrapAngle(t * 2 * .pi * 1.6 + member.wobblePhase),
                size: school.size * member.size,
                colour: school.tint)
        }
    }

    // MARK: - Echoes

    func echoPose(for echo: Echo, at t: Double) -> EchoPose {
        let left = echo.leftAt.timeIntervalSince1970
        // The echo stays where the whale swam in this cove when it left, as the camera saw it.
        let place = WhalePath(cove: seed, creatureID: echo.creatureID, look: echo.look).state(at: left).position
        let seen = SceneCamera.project(place, aspect: SceneRules.worldWidth)
        let position = (
            x: SceneRules.worldCentreX + ((seen?.x ?? 0.5) - 0.5) * SceneRules.worldWidth,
            y: min(max(seen?.y ?? 0.5, 0.15), 0.85),
            depth: min(max((seen?.distance ?? 40) / 90, 0.1), 0.9))
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


/// The camera on the seabed, for working out where something in the water appears in the picture.
public enum SceneCamera {
    /// Where a point in the water (metres) appears: x and y run 0...1 across and up the picture (outside that
    /// range the point is out of view), with its distance from the camera. Nil if it is behind the camera.
    public static func project(_ point: SIMD3<Double>, aspect: Double) -> (x: Double, y: Double, distance: Double)? {
        let d = point - SIMD3(0, -SceneRules.cameraDepth, 0)
        let cp = cos(SceneRules.cameraPitch), sp = sin(SceneRules.cameraPitch)
        let up = d.y * cp - d.z * sp
        let ahead = d.y * sp + d.z * cp
        guard ahead > 0.3 else { return nil }
        let f = SceneRules.cameraFocal
        return (0.5 + f * d.x / ahead / aspect, 0.5 + f * up / ahead, (d * d).sum().squareRoot())
    }
}
