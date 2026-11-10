import CoveScene
import Foundation

/// Made-up visitors and echoes for looking at the scene before the backend exists.
enum SampleCove {
    static func visitors(seed: UInt64, count: Int, now: Date) -> [Visitor] {
        (0..<count).map { i in
            Visitor(
                creatureID: "sample-\(seed)-\(i)",
                look: WhaleLook(colour: Int(seed % 6) + i * 2, pattern: i, size: 0.85 + 0.12 * Double(i % 3)),
                // The last one arrives a moment after start, to show the fade-in.
                arrivedAt: i == count - 1 ? now.addingTimeInterval(5) : now.addingTimeInterval(-3600 * Double(i + 1)))
        }
    }

    static func echoes(seed: UInt64, count: Int, now: Date) -> [Echo] {
        (0..<count).map { i in
            Echo(
                creatureID: "echo-\(seed)-\(i)",
                look: WhaleLook(colour: i, pattern: 0, size: 1),
                leftAt: now.addingTimeInterval(-86_400 * (Double(i) + 0.3)))
        }
    }
}
