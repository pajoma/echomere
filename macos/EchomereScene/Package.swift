// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "EchomereScene",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "CoveScene", targets: ["CoveScene"]),
        .library(name: "CoveSceneRenderer", targets: ["CoveSceneRenderer"]),
    ],
    targets: [
        // The deterministic scene: (cove seed, visitors, echoes, time) -> what is on screen. No Metal.
        .target(name: "CoveScene"),
        // Draws a scene frame with Metal.
        .target(name: "CoveSceneRenderer", dependencies: ["CoveScene"]),
        // A window to look at the scene, and a snapshot tool.
        .executableTarget(name: "ScenePreview", dependencies: ["CoveScene", "CoveSceneRenderer"]),
        // Behaviour checks at the scene seam. Runs without Xcode: `swift run SceneChecks`.
        .executableTarget(name: "SceneChecks", dependencies: ["CoveScene", "CoveSceneRenderer"]),
    ]
)
