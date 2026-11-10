import CoreGraphics
import CoveScene
import Foundation
import ImageIO
import Metal
import QuartzCore
import UniformTypeIdentifiers

public enum CoveSceneRendererError: Error {
    case noCommandQueue
    case missingShader(String)
    case cannotCreate(String)
}

/// Draws scene frames with Metal: into a `CAMetalLayer` for the screensaver, or into an image for snapshots.
public final class CoveSceneRenderer {
    public let device: MTLDevice
    public let pixelFormat: MTLPixelFormat
    private let queue: MTLCommandQueue
    private let pipeline: MTLRenderPipelineState
    private let wavePipeline: MTLComputePipelineState
    /// The sea's wave field on one 32 m patch, recomputed every frame: height and slope, and curvature.
    private let heights: MTLTexture
    private let curvature: MTLTexture
    static let waveFieldSize = 256

    public init(device: MTLDevice, pixelFormat: MTLPixelFormat = .bgra8Unorm) throws {
        self.device = device
        self.pixelFormat = pixelFormat
        let library = try device.makeLibrary(source: SceneShaders.source, options: nil)
        guard let vertex = library.makeFunction(name: "scene_vertex") else {
            throw CoveSceneRendererError.missingShader("scene_vertex")
        }
        guard let fragment = library.makeFunction(name: "scene_fragment") else {
            throw CoveSceneRendererError.missingShader("scene_fragment")
        }
        let descriptor = MTLRenderPipelineDescriptor()
        descriptor.vertexFunction = vertex
        descriptor.fragmentFunction = fragment
        descriptor.colorAttachments[0].pixelFormat = pixelFormat
        pipeline = try device.makeRenderPipelineState(descriptor: descriptor)
        guard let waveFunction = library.makeFunction(name: "wave_field") else {
            throw CoveSceneRendererError.missingShader("wave_field")
        }
        wavePipeline = try device.makeComputePipelineState(function: waveFunction)

        let field = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: .rgba16Float, width: Self.waveFieldSize, height: Self.waveFieldSize, mipmapped: false)
        field.usage = [.shaderRead, .shaderWrite]
        field.storageMode = .private
        guard let heights = device.makeTexture(descriptor: field),
              let curvature = device.makeTexture(descriptor: field)
        else { throw CoveSceneRendererError.cannotCreate("wave field textures") }
        self.heights = heights
        self.curvature = curvature

        guard let queue = device.makeCommandQueue() else { throw CoveSceneRendererError.noCommandQueue }
        self.queue = queue
    }

    /// Draws one frame into the layer's next drawable. Does nothing when no drawable is available.
    public func draw(_ frame: SceneFrame, in layer: CAMetalLayer) {
        guard let drawable = layer.nextDrawable(), let commandBuffer = queue.makeCommandBuffer() else { return }
        encode(frame, into: drawable.texture, commandBuffer: commandBuffer)
        commandBuffer.present(drawable)
        commandBuffer.commit()
    }

    /// Renders one frame offscreen and returns it as an image.
    public func snapshot(_ frame: SceneFrame, width: Int, height: Int) throws -> CGImage {
        let descriptor = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: .bgra8Unorm, width: width, height: height, mipmapped: false)
        descriptor.usage = [.renderTarget, .shaderRead]
        descriptor.storageMode = device.hasUnifiedMemory ? .shared : .managed
        guard let texture = device.makeTexture(descriptor: descriptor),
              let commandBuffer = queue.makeCommandBuffer()
        else { throw CoveSceneRendererError.cannotCreate("snapshot texture") }

        encode(frame, into: texture, commandBuffer: commandBuffer)
        if texture.storageMode == .managed, let blit = commandBuffer.makeBlitCommandEncoder() {
            blit.synchronize(resource: texture)
            blit.endEncoding()
        }
        commandBuffer.commit()
        commandBuffer.waitUntilCompleted()

        let bytesPerRow = width * 4
        var pixels = [UInt8](repeating: 0, count: bytesPerRow * height)
        texture.getBytes(&pixels, bytesPerRow: bytesPerRow,
                         from: MTLRegionMake2D(0, 0, width, height), mipmapLevel: 0)
        guard let provider = CGDataProvider(data: Data(pixels) as CFData),
              let image = CGImage(
                  width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: bytesPerRow,
                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                  bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipFirst.rawValue
                      | CGBitmapInfo.byteOrder32Little.rawValue),
                  provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent)
        else { throw CoveSceneRendererError.cannotCreate("snapshot image") }
        return image
    }

    private func encode(_ frame: SceneFrame, into texture: MTLTexture, commandBuffer: MTLCommandBuffer) {
        encodeWaveField(frame.waves, commandBuffer: commandBuffer)
        let data = SceneBuffer.pack(
            frame, aspect: Double(texture.width) / Double(texture.height), pixelsHigh: Double(texture.height))
        guard let buffer = data.withUnsafeBytes({ bytes in
            device.makeBuffer(bytes: bytes.baseAddress!, length: bytes.count, options: .storageModeShared)
        }) else { return }

        let pass = MTLRenderPassDescriptor()
        pass.colorAttachments[0].texture = texture
        pass.colorAttachments[0].loadAction = .dontCare
        pass.colorAttachments[0].storeAction = .store
        guard let encoder = commandBuffer.makeRenderCommandEncoder(descriptor: pass) else { return }
        encoder.setRenderPipelineState(pipeline)
        encoder.setFragmentBuffer(buffer, offset: 0, index: 0)
        encoder.setFragmentTexture(heights, index: 0)
        encoder.setFragmentTexture(curvature, index: 1)
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
        encoder.endEncoding()
    }
}

extension CoveSceneRenderer {
    fileprivate func encodeWaveField(_ waves: [Wave], commandBuffer: MTLCommandBuffer) {
        guard let encoder = commandBuffer.makeComputeCommandEncoder() else { return }
        var data = SceneBuffer.packWaves(waves)
        encoder.setComputePipelineState(wavePipeline)
        encoder.setTexture(heights, index: 0)
        encoder.setTexture(curvature, index: 1)
        encoder.setBytes(&data, length: MemoryLayout<SIMD4<Float>>.stride * data.count, index: 0)
        let group = MTLSize(width: 16, height: 16, depth: 1)
        let groups = MTLSize(width: Self.waveFieldSize / 16, height: Self.waveFieldSize / 16, depth: 1)
        encoder.dispatchThreadgroups(groups, threadsPerThreadgroup: group)
        encoder.endEncoding()
    }
}

/// Writes an image as PNG.
public func writePNG(_ image: CGImage, to url: URL) throws {
    guard let destination = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil)
    else { throw CoveSceneRendererError.cannotCreate("PNG at \(url.path)") }
    CGImageDestinationAddImage(destination, image, nil)
    guard CGImageDestinationFinalize(destination) else {
        throw CoveSceneRendererError.cannotCreate("PNG at \(url.path)")
    }
}
