import CoveScene
import CoveSceneRenderer
import Foundation
import Metal

// Behaviour checks at the scene seam (issue #7, testing seam 3). Runs without Xcode:
//   swift run SceneChecks
// Exits with status 1 if any check fails.

var failures: [String] = []
var passed = 0

@MainActor
func check(_ name: String, _ condition: Bool) {
    if condition {
        passed += 1
        print("  ok    \(name)")
    } else {
        failures.append(name)
        print("  FAIL  \(name)")
    }
}

let noon = Date(timeIntervalSince1970: 1_794_000_000)
let hour: TimeInterval = 3600
let day: TimeInterval = 24 * hour

func visitor(_ id: String, arrived: Date, species: WhaleSpecies = .southernRightWhale) -> Visitor {
    Visitor(creatureID: id, look: WhaleLook(species: species, size: 1), arrivedAt: arrived)
}

func echo(_ id: String, left: Date) -> Echo {
    Echo(creatureID: id, look: WhaleLook(species: .southernRightWhale, size: 1), leftAt: left)
}

let munichBay = CoveScene(seed: CoveSeed(4_711))
let guests = [
    visitor("whale-a", arrived: noon - 5 * hour),
    visitor("whale-b", arrived: noon - 2 * hour, species: .orca),
    visitor("whale-c", arrived: noon - hour),
]
let traces = [echo("whale-x", left: noon - day), echo("whale-y", left: noon - 3 * day)]

print("Every screen in a cove shows the same picture")
check("the same cove, visitors, echoes and time give the same frame",
      CoveScene(seed: CoveSeed(4_711)).frame(at: noon, visitors: guests, echoes: traces)
          == munichBay.frame(at: noon, visitors: guests, echoes: traces))
check("the order the visitors arrive in from the server doesn't change the frame",
      munichBay.frame(at: noon, visitors: guests.reversed(), echoes: traces.reversed())
          == munichBay.frame(at: noon, visitors: guests, echoes: traces))
check("different coves have different seabeds, kelp and fish",
      CoveScene(seed: CoveSeed(1)).layout != CoveScene(seed: CoveSeed(2)).layout)
check("the same whale swims a different path in another cove",
      CoveScene(seed: CoveSeed(1)).frame(at: noon, visitors: [guests[0]], echoes: []).whales
          != CoveScene(seed: CoveSeed(2)).frame(at: noon, visitors: [guests[0]], echoes: []).whales)

print("Visiting whales")
check("a whale isn't in the scene before it arrives",
      munichBay.frame(at: noon, visitors: [visitor("late", arrived: noon + 60)], echoes: []).whales.isEmpty)
let arriving = [visitor("new", arrived: noon)]
let opacities = [0.0, 60, 120, 600].map {
    munichBay.frame(at: noon + $0, visitors: arriving, echoes: []).whales.first?.opacity ?? -1
}
check("a whale fades in over two minutes after it arrives",
      opacities[0] == 0 && opacities[1] > 0.4 && opacities[1] < 0.6 && opacities[2] == 1 && opacities[3] == 1)
let crowd = (0..<8).map { visitor("crowd-\($0)", arrived: noon - Double(8 - $0) * hour) }
let crowded = munichBay.frame(at: noon, visitors: crowd, echoes: [])
check("at most six whales are drawn: five visitors and a creator's new whale",
      crowded.whales.count == SceneRules.maxWhales)

var inView = 0, samples = 0, upFront = 0
var fastest = 0.0, shallowest = -100.0, deepest = 0.0
var previous: WhalePose?
for step in stride(from: 0.0, to: day, by: 20) {
    guard let whale = munichBay.frame(at: noon + step, visitors: [guests[0]], echoes: []).whales.first else { continue }
    samples += 1
    if let seen = SceneCamera.project(whale.position, aspect: 1.6),
       seen.x > -0.1, seen.x < 1.1, seen.y > -0.1, seen.y < 1.1, seen.distance < 110 {
        inView += 1
    }
    if whale.position.z < 32 { upFront += 1 }
    shallowest = max(shallowest, whale.position.y)
    deepest = min(deepest, whale.position.y)
    if let previous {
        let d = whale.position - previous.position
        fastest = max(fastest, (d * d).sum().squareRoot() / 20)
    }
    previous = whale
}
let inViewShare = Double(inView) / Double(samples)
check("a whale roams in and out of view (in view \(Int(inViewShare * 100))% of the day)",
      inViewShare > 0.3 && inViewShare < 0.85)
let upFrontShare = Double(upFront) / Double(samples)
check("a whale comes to the front only rarely (\(String(format: "%.1f", upFrontShare * 100))% of the day)",
      upFront > 0 && upFrontShare < 0.05)
check("a whale stays between the seabed and the surface (\(Int(-shallowest)) to \(Int(-deepest)) m deep)",
      shallowest < -3 && deepest > -SceneRules.cameraDepth + 2)
check("a whale swims at a whale's pace (at most \(String(format: "%.1f", fastest)) m/s)", fastest < 2)
let whaleNow = munichBay.frame(at: noon, visitors: guests, echoes: []).whales
let whaleNext = munichBay.frame(at: noon + 1.0 / 15, visitors: guests, echoes: []).whales
check("whales move smoothly from one frame to the next",
      zip(whaleNow, whaleNext).allSatisfy { a, b in
          let d = a.position - b.position
          return a.id == b.id && (d * d).sum().squareRoot() < 0.2 && abs(a.roll - b.roll) < 0.05
      })
check("a visitor is the whale its creator chose",
      whaleNow.first { $0.id == "whale-b" }?.species == .orca && whaleNow.first { $0.id == "whale-b" }?.length == 7)

print("Echoes")
check("an echo older than seven days is gone",
      munichBay.frame(at: noon, visitors: [], echoes: [echo("old", left: noon - 7 * day - 1)]).echoes.isEmpty)
let manyEchoes = (0..<8).map { echo("e\($0)", left: noon - Double($0 + 1) * hour) }
let withEchoes = munichBay.frame(at: noon, visitors: [], echoes: manyEchoes)
check("a cove shows at most its five most recent echoes",
      withEchoes.echoes.count == SceneRules.maxEchoes
          && withEchoes.echoes.map(\.intensity) == munichBay.frame(
              at: noon, visitors: [], echoes: Array(manyEchoes.prefix(5))).echoes.map(\.intensity))
let fresh = munichBay.frame(at: noon, visitors: [], echoes: [echo("e", left: noon - hour)]).echoes[0].intensity
let older = munichBay.frame(at: noon, visitors: [], echoes: [echo("e", left: noon - 5 * day)]).echoes[0].intensity
check("an echo grows fainter as it ages", fresh > older && older > 0)
let calls = stride(from: 0.0, to: 2 * hour, by: 1).filter {
    munichBay.frame(at: noon + $0, visitors: [], echoes: [echo("e", left: noon - hour)]).echoes[0].call != nil
}.count
check("an echo calls now and then, not all the time (\(calls) s of calling in two hours)", calls > 0 && calls < 60)

print("The scene itself")
check("kelp is rooted below the view and reaches the surface",
      !munichBay.layout.kelp.isEmpty && munichBay.layout.kelp.allSatisfy {
          $0.baseDepth > SceneRules.cameraDepth && $0.length > $0.baseDepth
      })
let sway = Kelp.motion(at: noon), swayLater = Kelp.motion(at: noon + Kelp.surgePeriod / 4)
var quarter = swayLater.surge - sway.surge
if quarter < 0 { quarter += 2 * .pi }
check("kelp is only hinted at, at least 50 m from the camera",
      munichBay.layout.kelp.allSatisfy { $0.baseZ > 50 })
check("the small fish swim near the camera, in front of the whales",
      !munichBay.layout.schools.isEmpty && munichBay.layout.schools.allSatisfy { $0.depth < 0.4 })
check("kelp sways with the swell (one sway every \(String(format: "%.1f", Kelp.surgePeriod)) s)",
      abs(quarter - .pi / 2) < 1e-6 && Kelp.surgePeriod > 8 && Kelp.surgePeriod < 10)
check("kelp sways the same on every screen",
      munichBay.frame(at: noon, visitors: [], echoes: []).kelpMotion
          == CoveScene(seed: CoveSeed(9)).frame(at: noon, visitors: [], echoes: []).kelpMotion)
let frame = munichBay.frame(at: noon, visitors: guests, echoes: traces)
check("small fish are drawn from far to near",
      zip(frame.creatures, frame.creatures.dropFirst()).allSatisfy { $0.depth >= $1.depth })
let next = munichBay.frame(at: noon + 1.0 / 15, visitors: guests, echoes: traces)
let nextByID = Dictionary(uniqueKeysWithValues: next.creatures.map { ($0.id, $0) })
let largestStep = frame.creatures.compactMap { c in nextByID[c.id].map { hypot(c.x - $0.x, c.y - $0.y) } }.max() ?? 1
check("small fish move smoothly from one frame to the next", largestStep < 0.01 && nextByID.count == frame.creatures.count)
check("the scene has ambient life", !frame.fish.isEmpty)

print("The sea")
check("the sea loops seamlessly every \(Int(Sea.loopPeriod)) s",
      abs(Sea.time(at: noon) - Sea.time(at: noon + Sea.loopPeriod)) < 1e-6
          && abs(Sea.time(at: noon + 1) - Sea.time(at: noon) - 1) < 1e-6)
check("every cove has the same sea above it",
      CoveScene(seed: CoveSeed(1)).frame(at: noon, visitors: [], echoes: []).seaTime
          == CoveScene(seed: CoveSeed(2)).frame(at: noon, visitors: [], echoes: []).seaTime)

print("The sky")
let noonSky = Sky.state(hour: 12), midnightSky = Sky.state(hour: 0)
check("the sun is up at noon and the moon at midnight",
      !noonSky.isNight && noonSky.lightDirection.y > 0.8 && midnightSky.isNight && midnightSky.lightDirection.y > 0.6)
check("the night is darker than the day, but never black",
      midnightSky.ambient < noonSky.ambient / 2.5 && midnightSky.ambient > 0.2
          && midnightSky.lightIntensity < noonSky.lightIntensity)
check("neither sun nor moon shines while it is on the horizon",
      Sky.state(hour: 6).lightIntensity < 1e-9 && Sky.state(hour: 18).lightIntensity < 1e-9)
var largestJump = 0.0
for minute in 0..<(24 * 60) {
    let a = Sky.state(hour: Double(minute) / 60), b = Sky.state(hour: Double(minute + 1) / 60)
    largestJump = max(largestJump, abs(a.ambient - b.ambient), abs(a.lightIntensity - b.lightIntensity),
                      (a.zenith - b.zenith).max(), (a.horizon - b.horizon).max())
}
check("the light never jumps: it changes by at most \(String(format: "%.3f", largestJump)) per minute", largestJump < 0.05)
check("sun and moon stay ahead of the camera, within its lens",
      stride(from: 0.0, to: 24, by: 0.25).allSatisfy {
          let d = Sky.state(hour: $0).lightDirection
          return abs(atan2(d.x, d.z)) <= 41 * Double.pi / 180
      })
check("the scene uses the sky of its own time",
      munichBay.frame(at: noon, visitors: [], echoes: []).sky == Sky.state(at: noon))

print("Rendering")
if let device = MTLCreateSystemDefaultDevice(), let renderer = try? CoveSceneRenderer(device: device) {
    check("the shaders compile", true)
    if let stats = try? renderer.seaStatistics(seaTime: 123) {
        for s in stats {
            print(String(format: "        cascade %d: height %.3f m, slope %.3f, curvature %.3f /m",
                         s.cascade, s.heightRMS, s.slopeRMS, s.curvatureRMS))
        }
        let total = stats.map(\.heightRMS).reduce(0) { $0 + $1 * $1 }.squareRoot()
        check("the sea has the waves of a rough open sea (height RMS \(String(format: "%.2f", total)) m)",
              total > 0.5 && total < 2.0)
    } else {
        check("the sea can be measured", false)
    }
    if let first = try? renderer.snapshot(frame, width: 320, height: 200),
       let second = try? renderer.snapshot(frame, width: 320, height: 200),
       let firstData = first.dataProvider?.data as Data?,
       let secondData = second.dataProvider?.data as Data? {
        check("the same frame renders to the same pixels", firstData == secondData)
        var sum = 0.0
        for i in stride(from: 0, to: firstData.count, by: 4) {
            sum += 0.0722 * Double(firstData[i]) + 0.7152 * Double(firstData[i + 1]) + 0.2126 * Double(firstData[i + 2])
        }
        let brightness = sum / Double(firstData.count / 4) / 255
        check("the picture is neither black nor washed out (mean brightness \(String(format: "%.3f", brightness)))",
              brightness > 0.05 && brightness < 0.35)
    } else {
        check("a snapshot can be rendered", false)
    }
} else {
    check("the shaders compile", false)
}

print("\n\(passed) passed, \(failures.count) failed")
exit(failures.isEmpty ? 0 : 1)
