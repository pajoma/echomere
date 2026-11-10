import CoveScene
import Foundation
import Metal
import MetalKit

/// A whale model on the GPU: its mesh and its skin texture, loaded from Resources/Whales.
/// The files are made by tools/prepare-whales.py; see that script for the mesh format.
final class WhaleModel {
    let vertices: MTLBuffer
    let indices: MTLBuffer
    let indexCount: Int
    let skin: MTLTexture

    static func fileName(for species: WhaleSpecies) -> String {
        switch species {
        case .southernRightWhale: "southern-right-whale"
        case .orca: "orca"
        }
    }

    init(species: WhaleSpecies, device: MTLDevice) throws {
        let name = Self.fileName(for: species)
        guard let meshURL = Bundle.module.url(forResource: name, withExtension: "mesh", subdirectory: "Whales"),
              let skinURL = Bundle.module.url(forResource: name, withExtension: "jpg", subdirectory: "Whales")
        else { throw CoveSceneRendererError.cannotCreate("whale model \(name): files not found") }

        let data = try Data(contentsOf: meshURL)
        guard data.count >= 12, data.prefix(4) == Data("EWM1".utf8) else {
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
    }

    /// Position (3 floats), normal (3 floats), texture coordinate (2 floats).
    static let vertexStride = 8 * MemoryLayout<Float>.stride

    static var vertexDescriptor: MTLVertexDescriptor {
        let d = MTLVertexDescriptor()
        d.attributes[0].format = .float3
        d.attributes[0].offset = 0
        d.attributes[1].format = .float3
        d.attributes[1].offset = 12
        d.attributes[2].format = .float2
        d.attributes[2].offset = 24
        d.layouts[0].stride = vertexStride
        return d
    }
}
