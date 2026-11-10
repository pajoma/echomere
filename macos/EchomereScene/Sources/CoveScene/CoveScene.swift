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
            time: time, whales: whales, creatures: creatures, kelp: layout.kelp,
            kelpMotion: Kelp.motion(at: time), echoes: Array(recentEchoes),
            seaTime: Sea.time(at: time), sky: Sky.state(at: time))
    }

    // MARK: - Whales

    /// A whale roams the cove on a slow, endless path. It swims mostly far from the camera and only rarely, on
    /// a slow cycle, comes to the front. It dives, each time to a different depth, and comes up to the surface
    /// to take a series of breaths before it dives again.
    ///
    /// A right whale has a path and a breathing rhythm of its own. Orcas travel as a pod: every orca in a cove
    /// follows the cove's one pod route and breathes with the pod, in a place of its own within it.
    struct WhalePath {
        /// Where the whale or its pod goes, and when it breathes.
        struct Route {
            let seed: UInt64
            let amplitudeX, periodX, phaseX: Double
            let centreZ, amplitudeZ, phaseZ: Double
            let periodY, phaseY: Double
            /// The whale comes to the front once in this many rounds of its route.
            let roundsPerVisit: Double
            let nearest: Double
            let breathingCycle, breathingOffset, breathInterval: Double
            let breaths: Int

            init(seed: UInt64, species: WhaleSpecies) {
                self.seed = seed
                var random = SplitMix64(seed: seed)
                amplitudeX = random.range(95, 125)
                periodX = random.range(520, 760) / species.pace
                phaseX = random.angle()
                centreZ = random.range(62, 76)
                amplitudeZ = random.range(14, 20)
                // Kept away from a quarter turn, where the figure of eight would have a stopping point.
                phaseZ = random.range(-0.9, 0.9)
                periodY = random.range(240, 420)
                phaseY = random.angle()
                roundsPerVisit = max(3, (random.range(3600, 7200) / periodX).rounded())
                nearest = random.range(14, 22)
                breathingCycle = random.range(species.breathingCycle.lowerBound, species.breathingCycle.upperBound)
                breathingOffset = random.range(0, breathingCycle)
                breaths = random.int(species.breaths.lowerBound, species.breaths.upperBound)
                breathInterval = species.breathInterval * random.range(0.9, 1.1)
            }

            /// Where the route is over the ground (x, z) at a moment. The route is a wide figure of eight,
            /// traced by one clock, so the whale never comes to a stop and always has a direction.
            func ground(at t: Double) -> SIMD2<Double> {
                let around = cycle(t, periodX) + phaseX
                // The visit to the front: 0 for most of the time, rising to 1 for a short while. It peaks when
                // the whale crosses in front of the camera at its full sideways speed, so the whale swims past
                // the camera; it never comes to it, stops and backs away.
                // The whole visit fits into that one crossing: it is over before the whale reaches the far ends
                // of its route, where it turns.
                var long = cycle(t, periodX * roundsPerVisit) + phaseX / roundsPerVisit
                long = long.truncatingRemainder(dividingBy: 2 * .pi)
                if long > .pi { long -= 2 * .pi }
                let fromCrossing = long * roundsPerVisit            // in radians of the route
                let visit = exp(-(fromCrossing / 0.55) * (fromCrossing / 0.55))
                let far = centreZ + amplitudeZ * sin(2 * around + phaseZ)
                return SIMD2(amplitudeX * (1 - 0.6 * visit) * sin(around), far + (nearest - far) * visit)
            }
        }

        let species: WhaleSpecies
        let route: Route
        /// An orca's place in its pod: seconds behind the lead, metres from the pod's line (in a fixed
        /// direction), metres above or below, and seconds after the pod's breath. All zero for a right whale.
        let behind, beside, besideAngle, above, breathDelay: Double
        /// A small wander of its own around its place.
        let wanderPeriod, wanderPhase: Double
        /// Depth of the whale's middle when its back breaks the surface, in metres.
        let surfaceDepth: Double
        let tailPeriod, tailPhase: Double
        let rollPeriod, rollPhase: Double

        init(cove: CoveSeed, creatureID: String, look: WhaleLook) {
            species = look.species
            let own = combine(cove.value, stableHash(creatureID))
            var random = SplitMix64(seed: combine(own, 0x9E37))
            if look.species.travelsInPods {
                // One route per cove and species, so every orca in the cove shares it.
                route = Route(seed: combine(cove.value, stableHash("pod-\(look.species.rawValue)")), species: look.species)
                behind = random.range(0, 30)
                beside = random.range(3, 12)
                besideAngle = random.angle()
                above = random.range(-2, 2)
                breathDelay = random.range(0, 4)
            } else {
                route = Route(seed: own, species: look.species)
                behind = 0; beside = 0; besideAngle = 0; above = 0; breathDelay = 0
            }
            wanderPeriod = random.range(110, 170)
            wanderPhase = random.angle()
            surfaceDepth = 0.085 * look.length
            tailPeriod = random.range(4.5, 6.5)
            tailPhase = random.angle()
            rollPeriod = random.range(70, 130)
            rollPhase = random.angle()
        }

        /// Orcas travel more slowly at night than by day: their clock runs 13 % fast at noon and 13 % slow at
        /// midnight, without ever jumping. Right whales keep the same pace.
        func travelTime(_ t: Double) -> Double {
            guard species == .orca else { return t }
            let day = 86_400.0
            return t - 0.13 * day / (2 * .pi) * sin(cycle(t, day))
        }

        /// How close the whale is to its breathing at the surface (1) or to its dive (0), with the depth it
        /// keeps in each.
        func breathing(at t: Double) -> (atSurface: Double, surfaceY: Double, diveY: Double) {
            let cycles = (t - breathDelay + route.breathingOffset) / route.breathingCycle
            let phase = cycles - cycles.rounded(.down)
            let signed = phase > 0.5 ? phase - 1 : phase            // the surface series is centred on 0
            let half = Double(route.breaths) * route.breathInterval / 2 / route.breathingCycle
            let ramp = min(45, route.breathingCycle * 0.2) / route.breathingCycle   // going up or down
            let atSurface = 1 - Sky.smooth(half, half + ramp, abs(signed))

            // Between breaths the whale dips a little; at each breath its back breaks the surface.
            let u = min(max((signed + half) / (2 * half), 0), 1)
            let dip = 0.5 + 0.5 * cos(2 * Double.pi * Double(route.breaths) * u)
            let surfaceY = -surfaceDepth * (1 + 2.2 * dip)

            // Each dive goes to its own depth. A cycle runs from the middle of one surface series to the middle
            // of the next, so the depth changes while the whale is at the surface, where it doesn't matter.
            let dive = UInt64(max(cycles.rounded(.down), 0))
            var random = SplitMix64(seed: combine(route.seed, dive))
            let depth = random.unit()
            var diveCentre = -22 + 12 * depth
            if species == .orca {
                // Orcas dive deep far less often at night.
                let daylight = Sky.smooth(-6, 10, Sky.sunElevation(hour: Sky.hour(at: Date(timeIntervalSince1970: t))))
                diveCentre = (-14 + 8 * depth) * (1 - daylight) + diveCentre * daylight
            }
            let diveY = diveCentre + above + 3.5 * sin(cycle(t, route.periodY) + route.phaseY)
            return (atSurface, surfaceY, diveY)
        }

        func position(at t: Double) -> SIMD3<Double> {
            let along = travelTime(t) - behind
            var ground = route.ground(at: along)
            ground += SIMD2(cos(besideAngle), sin(besideAngle)) * beside
            let wander = cycle(t, wanderPeriod) + wanderPhase
            // Gentler than the slowest part of the route, so it can never bring the whale to a stop.
            ground += SIMD2(0.5 * sin(wander), 0.5 * sin(wander + 1.3))
            let breath = breathing(at: t)
            return SIMD3(ground.x, breath.diveY + (breath.surfaceY - breath.diveY) * breath.atSurface, ground.y)
        }

        /// Velocity in metres per second, measured over two seconds so it is steady.
        func velocity(at t: Double) -> SIMD3<Double> {
            (position(at: t + 1) - position(at: t - 1)) / 2
        }

        /// The compass direction the whale points in, in radians: the direction it swims in, averaged over
        /// about 16 seconds. So where its path doubles back and it almost stops, it comes round slowly, the
        /// way a whale turns, without spinning on the spot.
        func heading(at t: Double) -> Double {
            var sum = SIMD2<Double>(0, 0)
            for offset in stride(from: -8.0, through: 8.0, by: 2.0) {
                let v = velocity(at: t + offset)
                let ground = (v.x * v.x + v.z * v.z).squareRoot()
                if ground > 1e-9 { sum += SIMD2(v.x, v.z) / ground }
            }
            return atan2(sum.x, sum.y)
        }
    }

    func whalePose(for visitor: Visitor, at t: Double) -> WhalePose {
        let path = WhalePath(cove: seed, creatureID: visitor.creatureID, look: visitor.look)
        let v = path.velocity(at: t)
        let ground = (v.x * v.x + v.z * v.z).squareRoot()
        let speed = (v * v).sum().squareRoot()
        let heading = path.heading(at: t)
        // A whale tilts up when it climbs and down when it dives, but never steeply.
        let pitch = min(max(atan2(v.y, max(ground, 0.45)), -0.5), 0.5)
        // It banks into its turns, more the faster it turns, and rolls a little, lazily, as it swims.
        var turn = path.heading(at: t + 1) - path.heading(at: t - 1)
        if turn > .pi { turn -= 2 * .pi }
        if turn < -.pi { turn += 2 * .pi }
        let bank = min(max(turn / 2 * 4, -0.3), 0.3) * Sky.smooth(0.15, 0.4, ground)
        let sinceArrival = t - visitor.arrivedAt.timeIntervalSince1970
        return WhalePose(
            id: visitor.creatureID,
            species: visitor.look.species,
            position: path.position(at: t),
            forward: SIMD3(sin(heading) * cos(pitch), sin(pitch), cos(heading) * cos(pitch)),
            roll: bank + 0.12 * sin(cycle(t, path.rollPeriod) + path.rollPhase),
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
        let place = WhalePath(cove: seed, creatureID: echo.creatureID, look: echo.look).position(at: left)
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
        // Right whales call most at dusk and at night: then the echo calls twice as often.
        let night = echo.look.species == .southernRightWhale
            && Sky.sunElevation(hour: Sky.hour(at: Date(timeIntervalSince1970: t))) < 4
        let secondCall = (t - left + callOffset + callPeriod / 2).truncatingRemainder(dividingBy: callPeriod)
        let width = SceneRules.worldWidth
        return EchoPose(
            x: min(max(position.x, 0.1 * width), 0.9 * width),
            y: position.y,
            depth: position.depth,
            intensity: max(0, 1 - (t - left) / SceneRules.echoLifetime),
            call: sinceCall < callDuration ? sinceCall / callDuration
                : (night && secondCall < callDuration ? secondCall / callDuration : nil),
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
