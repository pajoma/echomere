/// The mesh of one kelp stalk with its blades. The vertices only say where on the stalk they belong; the
/// vertex shader (`kelp_vertex`) puts them in the water, so one mesh serves every stalk.
///
/// Each vertex is (s, u, v, blade):
/// - stalk: s runs 0...1 from root to tip, v is -1 or 1 across the ribbon, blade is -1.
/// - blade: u runs 0...1 from base to tip, v runs -1...1 across, blade is the blade's index.
enum KelpMesh {
    static let stalkSegments = 48
    /// Must match `BLADES` in the shader.
    static let blades = 64
    static let bladeRows = 9

    static func build() -> (vertices: [SIMD4<Float>], indices: [UInt16]) {
        var vertices: [SIMD4<Float>] = []
        var indices: [UInt16] = []

        for i in 0...stalkSegments {
            let s = Float(i) / Float(stalkSegments)
            vertices.append(SIMD4(s, 0, -1, -1))
            vertices.append(SIMD4(s, 0, 1, -1))
        }
        for i in 0..<stalkSegments {
            let a = UInt16(i * 2)
            indices += [a, a + 1, a + 2, a + 1, a + 3, a + 2]
        }

        for blade in 0..<blades {
            let first = UInt16(vertices.count)
            for row in 0..<bladeRows {
                let u = Float(row) / Float(bladeRows - 1)
                for v in [Float(-1), 0, 1] {
                    vertices.append(SIMD4(0, u, v, Float(blade)))
                }
            }
            for row in 0..<(bladeRows - 1) {
                for column in 0..<2 {
                    let a = first + UInt16(row * 3 + column)
                    indices += [a, a + 1, a + 3, a + 1, a + 4, a + 3]
                }
            }
        }
        return (vertices, indices)
    }
}
