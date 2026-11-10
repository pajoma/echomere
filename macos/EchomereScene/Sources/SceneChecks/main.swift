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

func visitor(_ id: String, arrived: Date, colour: Int = 0) -> Visitor {
    Visitor(creatureID: id, look: WhaleLook(colour: colour, pattern: 0, size: 1), arrivedAt: arrived)
}

func echo(_ id: String, left: Date) -> Echo {
    Echo(creatureID: id, look: WhaleLook(colour: 0, pattern: 0, size: 1), leftAt: left)
}

let munichBay = CoveScene(seed: CoveSeed(4_711))
let guests = [
    visitor("whale-a", arrived: noon - 5 * hour, colour: 1),
    visitor("whale-b", arrived: noon - 2 * hour, colour: 3),
    visitor("whale-c", arrived: noon - hour, colour: 5),
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

var visibleSamples = 0
var samples = 0
for step in stride(from: 0.0, to: day, by: 30) {
    samples += 1
    if let whale = munichBay.frame(at: noon + step, visitors: [guests[0]], echoes: []).whales.first,
       whale.x > -0.2, whale.x < SceneRules.worldWidth + 0.2 {
        visibleSamples += 1
    }
}
let visibleShare = Double(visibleSamples) / Double(samples)
check("a whale leaves the view now and then and comes back (in view \(Int(visibleShare * 100))% of the day)",
      visibleShare > 0.4 && visibleShare < 0.85)

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
check("kelp grows up into the view from the seabed below it",
      !munichBay.layout.kelp.isEmpty && munichBay.layout.kelp.allSatisfy { $0.rootY < 0 && $0.rootY + $0.height > 0.2 })
let frame = munichBay.frame(at: noon, visitors: guests, echoes: traces)
check("creatures are drawn from far to near",
      zip(frame.creatures, frame.creatures.dropFirst()).allSatisfy { $0.depth >= $1.depth })
let next = munichBay.frame(at: noon + 1.0 / 15, visitors: guests, echoes: traces)
let nextByID = Dictionary(uniqueKeysWithValues: next.creatures.map { ($0.id, $0) })
let largestStep = frame.creatures.compactMap { c in nextByID[c.id].map { hypot(c.x - $0.x, c.y - $0.y) } }.max() ?? 1
check("creatures move smoothly from one frame to the next", largestStep < 0.01 && nextByID.count == frame.creatures.count)
check("the scene has ambient life", !frame.fish.isEmpty)

print("The sea")
check("the sea loops seamlessly every \(Int(Sea.loopPeriod)) s",
      abs(Sea.time(at: noon) - Sea.time(at: noon + Sea.loopPeriod)) < 1e-6
          && abs(Sea.time(at: noon + 1) - Sea.time(at: noon) - 1) < 1e-6)
check("every cove has the same sea above it",
      CoveScene(seed: CoveSeed(1)).frame(at: noon, visitors: [], echoes: []).seaTime
          == CoveScene(seed: CoveSeed(2)).frame(at: noon, visitors: [], echoes: []).seaTime)

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
