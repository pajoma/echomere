import CoveScene
import Foundation
import Metal
import MetalKit

/// A whale model on the GPU: its mesh, its skin texture and its swim animation, loaded from Resources/Whales.
/// The files are made by tools/prepare-whales.py; see that script for the file formats.
final class WhaleModel {
    let vertices: MTLBuffer
    let indices: MTLBuffer
    let indexCount: Int
    let skin: MTLTexture
    /// The model's own swim animation: for every frame, every joint's matrix (three rows of four) from the
    /// bind pose to the swimming whale. It loops.
    let jointCount: Int
    let frameCount: Int
    private let frames: [SIMD4<Float>]
    /// The average pose over the loop: what the whale looks like when it glides.
    private let glide: [SIMD4<Float>]

    static func fileName(for species: WhaleSpecies) -> String {
        switch species {
        case .southernRightWhale: "southern-right-whale"
        case .orca: "orca"
        }
    }

    init(species: WhaleSpecies, device: MTLDevice) throws {
        let name = Self.fileName(for: species)
        guard let meshURL = Bundle.module.url(forResource: name, withExtension: "mesh", subdirectory: "Whales"),
              let skinURL = Bundle.module.url(forResource: name, withExtension: "jpg", subdirectory: "Whales"),
              let animationURL = Bundle.module.url(forResource: name, withExtension: "animation", subdirectory: "Whales")
        else { throw CoveSceneRendererError.cannotCreate("whale model \(name): files not found") }

        let data = try Data(contentsOf: meshURL)
        guard data.count >= 12, data.prefix(4) == Data("EWM2".utf8) else {
            throw CoveSceneRendererError.cannotCreate("whale model \(name): not a mesh file")
        }
        let (vertexCount, indexCount) = data.withUnsafeBytes { bytes in
            (Int(bytes.loadUnaligned(fromByteOffset: 4, as: UInt32.self)),
             Int(bytes.loadUnaligned(fromByteOffset: 8, as: UInt32.self)))
        }
        let vertexBytes = vertexCount * Self.vertexStride
        let indexBytes = indexCount * MemoryLayout<UInt32>.stride
        guard data.count == 12 + vertexBytes + indexBytes else {
            throw CoveSceneRendererError.cannotCreate("whale model \(name): wrong size")
        }
        let buffers: (MTLBuffer?, MTLBuffer?) = data.withUnsafeBytes { bytes in
            (device.makeBuffer(bytes: bytes.baseAddress! + 12, length: vertexBytes),
             device.makeBuffer(bytes: bytes.baseAddress! + 12 + vertexBytes, length: indexBytes))
        }
        guard let vertices = buffers.0, let indices = buffers.1 else {
            throw CoveSceneRendererError.cannotCreate("whale model \(name): buffers")
        }
        self.vertices = vertices
        self.indices = indices
        self.indexCount = indexCount
        skin = try MTKTextureLoader(device: device).newTexture(
            URL: skinURL, options: [.SRGB: false, .generateMipmaps: true])

        let animation = try Data(contentsOf: animationURL)
        guard animation.count >= 16, animation.prefix(4) == Data("EWA1".utf8) else {
            throw CoveSceneRendererError.cannotCreate("whale model \(name): not an animation file")
        }
        let (joints, frameCount) = animation.withUnsafeBytes { bytes in
            (Int(bytes.loadUnaligned(fromByteOffset: 4, as: UInt32.self)),
             Int(bytes.loadUnaligned(fromByteOffset: 8, as: UInt32.self)))
        }
        let rows = joints * frameCount * 3
        guard joints > 0, joints <= Self.maxJoints, frameCount > 0,
              animation.count == 16 + rows * MemoryLayout<SIMD4<Float>>.stride
        else { throw CoveSceneRendererError.cannotCreate("whale model \(name): wrong animation size") }
        var frames = [SIMD4<Float>](repeating: .zero, count: rows)
        frames.withUnsafeMutableBytes { target in
            animation.withUnsafeBytes { source in
                target.copyMemory(from: UnsafeRawBufferPointer(rebasing: source[16...]))
            }
        }
        self.jointCount = joints
        self.frameCount = frameCount
        self.frames = frames
        var glide = [SIMD4<Float>](repeating: .zero, count: joints * 3)
        for frame in 0..<frameCount {
            for row in 0..<(joints * 3) { glide[row] += frames[frame * joints * 3 + row] }
        }
        self.glide = glide.map { $0 / Float(frameCount) }
    }

    /// The most joints a model may have: their matrices are sent to the shader with every whale.
    static let maxJoints = 80

    /// The joint matrices for a moment in the swim loop.
    /// - Parameters:
    ///   - phase: where in the loop, 0..<1.
    ///   - effort: 1 swims fully; towards 0 the whale glides, holding the loop's average pose.
    func pose(phase: Double, effort: Double) -> [SIMD4<Float>] {
        let position = (phase - phase.rounded(.down)) * Double(frameCount)
        let first = Int(position) % frameCount
        let second = (first + 1) % frameCount
        let blend = Float(position - position.rounded(.down))
        let strength = Float(0.25 + 0.75 * min(max(effort, 0), 1))
        let rows = jointCount * 3
        return (0..<rows).map { row in
            let a = frames[first * rows + row], b = frames[second * rows + row]
            return glide[row] + (a + (b - a) * blend - glide[row]) * strength
        }
    }

    /// Position (3 floats), normal (3 floats), texture coordinate (2 floats), joints (4 uint16), weights (4
    /// floats).
    static let vertexStride = 8 * MemoryLayout<Float>.stride + 4 * MemoryLayout<UInt16>.stride
        + 4 * MemoryLayout<Float>.stride

    static var vertexDescriptor: MTLVertexDescriptor {
        let d = MTLVertexDescriptor()
        d.attributes[0].format = .float3
        d.attributes[0].offset = 0
        d.attributes[1].format = .float3
        d.attributes[1].offset = 12
        d.attributes[2].format = .float2
        d.attributes[2].offset = 24
        d.attributes[3].format = .ushort4
        d.attributes[3].offset = 32
        d.attributes[4].format = .float4
        d.attributes[4].offset = 40
        d.layouts[0].stride = vertexStride
        return d
    }
}
