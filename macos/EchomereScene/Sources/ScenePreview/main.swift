import AppKit
import CoveScene
import CoveSceneRenderer
import Foundation
import Metal
import QuartzCore

// Usage:
//   swift run ScenePreview [--seed N] [--whales N] [--echoes N] [--speed X] [--fps N] [--scale X]
//   swift run ScenePreview --snapshot out.png [--at UNIX_SECONDS] [--size 1600x1000] [--seed N] ...
//   swift run ScenePreview --frames DIR --seconds N [--fps N] [--at UNIX_SECONDS] [--size WxH] ...
//     (numbered PNGs; turn them into a clip with:
//      ffmpeg -framerate 15 -i DIR/frame-%05d.png -pix_fmt yuv420p clip.mp4)
//
// In the window: space shows another cove, Esc quits.

struct Options {
    var seed: UInt64 = 42
    var whales = 3
    var echoes = 2
    var speed = 1.0
    var fps = 15
    var renderScale = 0.5
    var snapshot: String?
    /// `--view above` shows the sea from above, for checking the waves.
    var seaFromAbove = false
    var framesDirectory: String?
    var seconds = 10.0
    var at: Date?
    var width = 1600
    var height = 1000

    init(arguments: [String]) {
        var it = arguments.dropFirst().makeIterator()
        while let flag = it.next() {
            let value = it.next() ?? ""
            switch flag {
            case "--seed": seed = UInt64(value) ?? seed
            case "--whales": whales = min(max(Int(value) ?? whales, 0), SceneRules.maxWhales)
            case "--echoes": echoes = min(max(Int(value) ?? echoes, 0), 8)
            case "--speed": speed = Double(value) ?? speed
            case "--fps": fps = min(max(Int(value) ?? fps, 1), 60)
            case "--scale": renderScale = min(max(Double(value) ?? renderScale, 0.1), 1)
            case "--view": seaFromAbove = value == "above"
            case "--snapshot": snapshot = value
            case "--frames": framesDirectory = value
            case "--seconds": seconds = Double(value) ?? seconds
            case "--at": at = Double(value).map { Date(timeIntervalSince1970: $0) }
            case "--size":
                let parts = value.split(separator: "x").compactMap { Int($0) }
                if parts.count == 2 { width = parts[0]; height = parts[1] }
            default:
                FileHandle.standardError.write(Data("Unknown option \(flag)\n".utf8))
                exit(2)
            }
        }
    }
}

let options = Options(arguments: CommandLine.arguments)

guard let device = MTLCreateSystemDefaultDevice() else {
    FileHandle.standardError.write(Data("No Metal device.\n".utf8))
    exit(1)
}
let renderer: CoveSceneRenderer
do {
    renderer = try CoveSceneRenderer(device: device)
} catch {
    FileHandle.standardError.write(Data("Renderer failed: \(error)\n".utf8))
    exit(1)
}

renderer.showsSeaFromAbove = options.seaFromAbove

if let directory = options.framesDirectory {
    let start = options.at ?? Date()
    let scene = CoveScene(seed: CoveSeed(options.seed))
    let visitors = SampleCove.visitors(seed: options.seed, count: options.whales, now: start.addingTimeInterval(-600))
    let echoes = SampleCove.echoes(seed: options.seed, count: options.echoes, now: start)
    let count = Int(options.seconds * Double(options.fps))
    do {
        try FileManager.default.createDirectory(atPath: directory, withIntermediateDirectories: true)
        for index in 0..<count {
            let time = start.addingTimeInterval(Double(index) / Double(options.fps) * options.speed)
            let image = try renderer.snapshot(
                scene.frame(at: time, visitors: visitors, echoes: echoes), width: options.width, height: options.height)
            try writePNG(image, to: URL(fileURLWithPath: directory)
                .appendingPathComponent(String(format: "frame-%05d.png", index)))
        }
        print("Wrote \(count) frames to \(directory)")
    } catch {
        FileHandle.standardError.write(Data("Frames failed: \(error)\n".utf8))
        exit(1)
    }
    exit(0)
}

if let path = options.snapshot {
    let time = options.at ?? Date()
    let scene = CoveScene(seed: CoveSeed(options.seed))
    let visitors = SampleCove.visitors(seed: options.seed, count: options.whales, now: time.addingTimeInterval(-600))
    let echoes = SampleCove.echoes(seed: options.seed, count: options.echoes, now: time)
    let frame = scene.frame(at: time, visitors: visitors, echoes: echoes)
    do {
        let image = try renderer.snapshot(frame, width: options.width, height: options.height)
        try writePNG(image, to: URL(fileURLWithPath: path))
        print("Wrote \(path): seed \(options.seed), \(frame.whales.count) whales, \(frame.fish.count) fish, "
            + "\(frame.kelp.count) kelp, \(frame.echoes.count) echoes")
    } catch {
        FileHandle.standardError.write(Data("Snapshot failed: \(error)\n".utf8))
        exit(1)
    }
    exit(0)
}

@MainActor
final class SceneView: NSView {
    private let renderer: CoveSceneRenderer
    private let options: Options
    private let metalLayer = CAMetalLayer()
    private var seed: UInt64
    private var scene: CoveScene
    private var visitors: [Visitor] = []
    private var echoes: [Echo] = []
    private let startedAt = Date()
    private var displayLink: CADisplayLink?

    init(renderer: CoveSceneRenderer, options: Options) {
        self.renderer = renderer
        self.options = options
        self.seed = options.seed
        self.scene = CoveScene(seed: CoveSeed(options.seed))
        super.init(frame: .zero)
        metalLayer.device = renderer.device
        metalLayer.pixelFormat = renderer.pixelFormat
        metalLayer.framebufferOnly = true
        metalLayer.isOpaque = true
        metalLayer.magnificationFilter = .linear
        wantsLayer = true
        loadCove()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    override func makeBackingLayer() -> CALayer { metalLayer }
    override var acceptsFirstResponder: Bool { true }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        displayLink?.invalidate()
        guard window != nil else { return }
        let link = displayLink(target: self, selector: #selector(tick(_:)))
        let fps = Float(options.fps)
        link.preferredFrameRateRange = CAFrameRateRange(minimum: fps, maximum: fps, preferred: fps)
        link.add(to: .main, forMode: .common)
        displayLink = link
        updateDrawableSize()
    }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        updateDrawableSize()
    }

    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        updateDrawableSize()
    }

    /// Renders below screen resolution to save energy; the layer scales it up, which suits the soft water.
    private func updateDrawableSize() {
        let backing = window?.backingScaleFactor ?? 2
        let scale = backing * options.renderScale
        metalLayer.contentsScale = backing
        metalLayer.drawableSize = CGSize(
            width: max(1, (bounds.width * scale).rounded()), height: max(1, (bounds.height * scale).rounded()))
    }

    private var sceneTime: Date {
        options.at ?? Date()
    }

    private func now() -> Date {
        sceneTime.addingTimeInterval(Date().timeIntervalSince(startedAt) * options.speed)
    }

    private func loadCove() {
        scene = CoveScene(seed: CoveSeed(seed))
        let now = now()
        visitors = SampleCove.visitors(seed: seed, count: options.whales, now: now)
        echoes = SampleCove.echoes(seed: seed, count: options.echoes, now: now)
        window?.title = "Echomere: cove \(seed)"
    }

    @objc private func tick(_ link: CADisplayLink) {
        // Stop drawing whenever nothing can see the window.
        guard let window, window.occlusionState.contains(.visible) else { return }
        renderer.draw(scene.frame(at: now(), visitors: visitors, echoes: echoes), in: metalLayer)
    }

    override func keyDown(with event: NSEvent) {
        switch event.keyCode {
        case 49:  // space
            seed &+= 1
            loadCove()
        case 53:  // escape
            NSApp.terminate(nil)
        default:
            super.keyDown(with: event)
        }
    }
}

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    let window: NSWindow

    init(view: SceneView) {
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1280, height: 800),
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered, defer: false)
        window.contentView = view
        window.title = "Echomere: cove \(options.seed)"
        window.center()
        super.init()
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(window.contentView)
        NSApp.activate()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}

let app = NSApplication.shared
app.setActivationPolicy(.regular)
let delegate = AppDelegate(view: SceneView(renderer: renderer, options: options))
app.delegate = delegate
app.run()
