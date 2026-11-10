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
        // Draws a scene frame with Metal. Resources/Whales holds the whale models, which have their own
        // licence (see Resources/Whales/LICENSE.md).
        .target(name: "CoveSceneRenderer", dependencies: ["CoveScene"], resources: [.copy("Resources/Whales")]),
        // A window to look at the scene, and a snapshot tool.
        .executableTarget(name: "ScenePreview", dependencies: ["CoveScene", "CoveSceneRenderer"]),
        // Behaviour checks at the scene seam. Runs without Xcode: `swift run SceneChecks`.
        .executableTarget(name: "SceneChecks", dependencies: ["CoveScene", "CoveSceneRenderer"]),
    ]
)
