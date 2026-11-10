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
///
/// Every frame first simulates the sea (an FFT ocean, see `OceanShaders`), then draws the scene in one
/// full-screen pass that reads the sea's slopes and curvature.
public final class CoveSceneRenderer {
    public let device: MTLDevice
    public let pixelFormat: MTLPixelFormat
    let queue: MTLCommandQueue
    private let pipeline: MTLRenderPipelineState
    let ocean: Ocean

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
        guard let queue = device.makeCommandQueue() else { throw CoveSceneRendererError.noCommandQueue }
        self.queue = queue
        ocean = try Ocean(device: device, queue: queue)
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
        ocean.simulate(seaTime: frame.seaTime, commandBuffer: commandBuffer)

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
        encoder.setFragmentTexture(ocean.displacement, index: 0)
        encoder.setFragmentTexture(ocean.derivatives, index: 1)
        encoder.setFragmentTexture(ocean.curvature, index: 2)
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
        encoder.endEncoding()
    }
}

/// Root-mean-square size of the sea's height, slope and curvature in one cascade.
public struct SeaStatistics: Sendable {
    public let cascade: Int
    public let heightRMS: Double
    public let slopeRMS: Double
    public let curvatureRMS: Double
}

extension CoveSceneRenderer {
    /// Simulates the sea at `seaTime` and measures it, for checks and tuning.
    public func seaStatistics(seaTime: Double) throws -> [SeaStatistics] {
        guard let commandBuffer = queue.makeCommandBuffer() else { throw CoveSceneRendererError.noCommandQueue }
        ocean.simulate(seaTime: seaTime, commandBuffer: commandBuffer)
        let n = OceanShaders.size
        let sliceBytes = n * n * 8
        func readback(_ texture: MTLTexture) -> MTLBuffer? {
            guard let buffer = device.makeBuffer(length: sliceBytes * OceanShaders.cascades, options: .storageModeShared),
                  let blit = commandBuffer.makeBlitCommandEncoder() else { return nil }
            for c in 0..<OceanShaders.cascades {
                blit.copy(from: texture, sourceSlice: c, sourceLevel: 0, sourceOrigin: MTLOrigin(x: 0, y: 0, z: 0),
                          sourceSize: MTLSize(width: n, height: n, depth: 1), to: buffer,
                          destinationOffset: c * sliceBytes, destinationBytesPerRow: n * 8,
                          destinationBytesPerImage: sliceBytes)
            }
            blit.endEncoding()
            return buffer
        }
        guard let displacement = readback(ocean.displacement), let derivatives = readback(ocean.derivatives),
              let curvature = readback(ocean.curvature)
        else { throw CoveSceneRendererError.cannotCreate("readback") }
        commandBuffer.commit()
        commandBuffer.waitUntilCompleted()

        func rms(_ buffer: MTLBuffer, cascade: Int, channels: [Int]) -> Double {
            let halves = buffer.contents().bindMemory(to: UInt16.self, capacity: sliceBytes / 2 * OceanShaders.cascades)
            var sum = 0.0
            for i in 0..<(n * n) {
                for ch in channels {
                    let v = Double(halfToFloat(halves[(cascade * n * n + i) * 4 + ch]))
                    sum += v * v
                }
            }
            return (sum / Double(n * n)).squareRoot()
        }
        return (0..<OceanShaders.cascades).map { c in
            SeaStatistics(cascade: c,
                          heightRMS: rms(displacement, cascade: c, channels: [1]),
                          slopeRMS: rms(derivatives, cascade: c, channels: [0, 1]),
                          curvatureRMS: rms(curvature, cascade: c, channels: [0, 2]))
        }
    }
}

/// IEEE 754 half precision to Float, without relying on Float16 (unavailable on Intel Macs).
func halfToFloat(_ bits: UInt16) -> Float {
    let sign: UInt32 = UInt32(bits & 0x8000) << 16
    let exponent = Int((bits >> 10) & 0x1F)
    let mantissa = UInt32(bits & 0x3FF)
    if exponent == 0 {
        let value = Float(mantissa) / 1024 * powf(2, -14)
        return sign == 0 ? value : -value
    }
    if exponent == 31 { return Float(bitPattern: sign | 0x7F80_0000 | (mantissa << 13)) }
    return Float(bitPattern: sign | (UInt32(exponent - 15 + 127) << 23) | (mantissa << 13))
}

/// Must match `OceanSettings` in `OceanShaders.source`.
struct OceanSettings {
    var L: SIMD4<Float>
    var kMin: SIMD4<Float>
    var kMax: SIMD4<Float>
    var wind: SIMD4<Float>
    var swell: SIMD4<Float>
    var misc: SIMD4<Float>
    var timeP: SIMD4<Float>

    static func from(seaTime: Double) -> OceanSettings {
        let n = Double(OceanShaders.size)
        let sizes = Sea.cascadeSizes
        var L = SIMD4<Float>(repeating: 1), kMin = SIMD4<Float>(repeating: 0), kMax = SIMD4<Float>(repeating: 0)
        for c in 0..<sizes.count {
            L[c] = Float(sizes[c])
            kMin[c] = Float(c == 0 ? Double.pi / sizes[c] : 2 * Double.pi * 6 / sizes[c])
            kMax[c] = Float(c < sizes.count - 1 ? 2 * Double.pi * 6 / sizes[c + 1] : Double.pi * n / sizes[c] * 0.98)
        }
        let degrees = Double.pi / 180
        let swellK = 2 * Double.pi / Sea.swellWavelength
        return OceanSettings(
            L: L, kMin: kMin, kMax: kMax,
            wind: SIMD4(Float(Sea.windSpeed), Float(Sea.windDirection * degrees), Float(Sea.fetch),
                        Float(Sea.jonswapGamma)),
            swell: SIMD4(Float(Sea.swellHeight), Float(Sea.swellDirection * degrees),
                         Float((9.81 * swellK).squareRoot()), Float(Sea.swellSpread)),
            misc: SIMD4(Float(Sea.choppiness), Float(Sea.shortWaveCut), Float(Sea.loopPeriod), Float(Sea.seed)),
            timeP: SIMD4(Float(seaTime), 0, 0, 1))
    }
}

/// The FFT ocean on the GPU: its spectrum (computed once), and per frame the displacement, slopes and
/// curvature of the surface for every cascade.
final class Ocean {
    let displacement: MTLTexture
    let derivatives: MTLTexture
    let curvature: MTLTexture
    private let h0: MTLBuffer
    private let specA: MTLBuffer
    private let specB: MTLBuffer
    private let rowsA: MTLBuffer
    private let rowsB: MTLBuffer
    private let evolve: MTLComputePipelineState
    private let rows: MTLComputePipelineState
    private let columns: MTLComputePipelineState
    private let curvaturePass: MTLComputePipelineState

    private static let n = OceanShaders.size
    private static let cascades = OceanShaders.cascades

    init(device: MTLDevice, queue: MTLCommandQueue) throws {
        let library = try device.makeLibrary(source: OceanShaders.source, options: nil)
        func pipeline(_ name: String) throws -> MTLComputePipelineState {
            guard let function = library.makeFunction(name: name) else {
                throw CoveSceneRendererError.missingShader(name)
            }
            return try device.makeComputePipelineState(function: function)
        }
        let initial = try pipeline("ocean_init")
        evolve = try pipeline("ocean_evolve")
        rows = try pipeline("ocean_fft_rows")
        columns = try pipeline("ocean_fft_cols")
        curvaturePass = try pipeline("ocean_curvature")

        let length = Self.cascades * Self.n * Self.n * MemoryLayout<SIMD4<Float>>.stride
        func buffer() throws -> MTLBuffer {
            guard let b = device.makeBuffer(length: length, options: .storageModePrivate) else {
                throw CoveSceneRendererError.cannotCreate("ocean buffer")
            }
            return b
        }
        h0 = try buffer(); specA = try buffer(); specB = try buffer(); rowsA = try buffer(); rowsB = try buffer()

        let descriptor = MTLTextureDescriptor()
        descriptor.textureType = .type2DArray
        descriptor.pixelFormat = .rgba16Float
        descriptor.width = Self.n
        descriptor.height = Self.n
        descriptor.arrayLength = Self.cascades
        descriptor.usage = [.shaderRead, .shaderWrite]
        descriptor.storageMode = .private
        func texture() throws -> MTLTexture {
            guard let t = device.makeTexture(descriptor: descriptor) else {
                throw CoveSceneRendererError.cannotCreate("ocean texture")
            }
            return t
        }
        displacement = try texture(); derivatives = try texture()
        // The curvature gets mipmaps: the light shafts read a blurred version of it.
        descriptor.mipmapLevelCount = 5
        curvature = try texture()

        // The spectrum never changes: compute it once.
        guard let commandBuffer = queue.makeCommandBuffer(),
              let encoder = commandBuffer.makeComputeCommandEncoder()
        else { throw CoveSceneRendererError.cannotCreate("ocean init") }
        var settings = OceanSettings.from(seaTime: 0)
        encoder.setComputePipelineState(initial)
        encoder.setBuffer(h0, offset: 0, index: 0)
        encoder.setBytes(&settings, length: MemoryLayout<OceanSettings>.stride, index: 1)
        encoder.dispatchThreadgroups(
            MTLSize(width: Self.n / 16, height: Self.n / 16, depth: Self.cascades),
            threadsPerThreadgroup: MTLSize(width: 16, height: 16, depth: 1))
        encoder.endEncoding()
        commandBuffer.commit()
        commandBuffer.waitUntilCompleted()
    }

    func simulate(seaTime: Double, commandBuffer: MTLCommandBuffer) {
        guard let encoder = commandBuffer.makeComputeCommandEncoder() else { return }
        var settings = OceanSettings.from(seaTime: seaTime)
        let settingsLength = MemoryLayout<OceanSettings>.stride
        let grid = MTLSize(width: Self.n / 16, height: Self.n / 16, depth: Self.cascades)
        let group = MTLSize(width: 16, height: 16, depth: 1)
        let fftGroups = MTLSize(width: Self.n, height: 1, depth: Self.cascades)
        let fftThreads = MTLSize(width: Self.n / 2, height: 1, depth: 1)

        encoder.setComputePipelineState(evolve)
        encoder.setBuffer(h0, offset: 0, index: 0)
        encoder.setBuffer(specA, offset: 0, index: 1)
        encoder.setBuffer(specB, offset: 0, index: 2)
        encoder.setBytes(&settings, length: settingsLength, index: 3)
        encoder.dispatchThreadgroups(grid, threadsPerThreadgroup: group)

        encoder.memoryBarrier(scope: .buffers)
        encoder.setComputePipelineState(rows)
        encoder.setBuffer(specA, offset: 0, index: 0)
        encoder.setBuffer(specB, offset: 0, index: 1)
        encoder.setBuffer(rowsA, offset: 0, index: 2)
        encoder.setBuffer(rowsB, offset: 0, index: 3)
        encoder.dispatchThreadgroups(fftGroups, threadsPerThreadgroup: fftThreads)

        encoder.memoryBarrier(scope: .buffers)
        encoder.setComputePipelineState(columns)
        encoder.setBuffer(rowsA, offset: 0, index: 0)
        encoder.setBuffer(rowsB, offset: 0, index: 1)
        encoder.setTexture(displacement, index: 0)
        encoder.setTexture(derivatives, index: 1)
        encoder.dispatchThreadgroups(fftGroups, threadsPerThreadgroup: fftThreads)

        encoder.memoryBarrier(scope: .textures)
        encoder.setComputePipelineState(curvaturePass)
        encoder.setTexture(derivatives, index: 0)
        encoder.setTexture(curvature, index: 1)
        encoder.setBytes(&settings, length: settingsLength, index: 0)
        encoder.dispatchThreadgroups(grid, threadsPerThreadgroup: group)
        encoder.endEncoding()

        guard let blit = commandBuffer.makeBlitCommandEncoder() else { return }
        blit.generateMipmaps(for: curvature)
        blit.endEncoding()
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
