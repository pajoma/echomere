#!/usr/bin/env python3
"""Prepares the whale models for the app.

Reads the original GLB downloads from assets/whales/source/ (not in the repository; see
assets/whales/README.md for where to get them) and writes, for each whale, a compact mesh and a reduced
texture to macos/EchomereScene/Sources/CoveSceneRenderer/Resources/Whales/.

The mesh is in the model's rest pose, turned so that x runs from tail (-0.5) to snout (+0.5), y is up and
z is to the whale's left, and scaled so the whale is one unit long. The app animates the tail itself.

Mesh file format (little-endian): "EWM1", vertex count (uint32), index count (uint32), then per vertex
eight float32 (position x y z, normal x y z, texture u v), then the indices as uint32.

Needs only the Python standard library, plus the macOS `sips` tool for the textures.
Usage: python3 tools/prepare-whales.py
"""
import json
import pathlib
import struct
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCE = ROOT / "assets" / "whales" / "source"
OUTPUT = ROOT / "macos" / "EchomereScene" / "Sources" / "CoveSceneRenderer" / "Resources" / "Whales"
TEXTURE_SIZE = 1024

# For each whale: the source file, which meshes to keep, which image is the body's colour, and how the
# model's axes map to ours: for each of our x, y, z, the (source axis, sign).
WHALES = {
    "southern-right-whale": {
        "file": "model_55a_-_southern_right_whale.glb",
        "meshes": ["Whale Bake"],
        "image": 4,
        "axes": [(1, -1), (2, 1), (0, -1)],
    },
    "orca": {
        "file": "female_orca.glb",
        "meshes": ["BODY_LOPOLY_ORCADIFFUSEONLY_0", "LEFT_EYEBALL_ORCADIFFUSEONLY_0",
                   "RIGHT_EYEBALL_ORCADIFFUSEONLY_0"],
        "image": 0,
        "axes": [(2, 1), (1, 1), (0, -1)],
    },
}

COMPONENTS = {5121: ("B", 1), 5123: ("H", 2), 5125: ("I", 4), 5126: ("f", 4)}
COUNTS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}


def read_glb(path):
    data = path.read_bytes()
    json_length = struct.unpack_from("<I", data, 12)[0]
    document = json.loads(data[20:20 + json_length])
    binary_start = 20 + json_length + 8
    return document, data[binary_start:]


def read_accessor(document, binary, index):
    accessor = document["accessors"][index]
    view = document["bufferViews"][accessor["bufferView"]]
    code, size = COMPONENTS[accessor["componentType"]]
    count = COUNTS[accessor["type"]]
    stride = view.get("byteStride") or size * count
    start = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
    rows = []
    for i in range(accessor["count"]):
        row = struct.unpack_from("<" + code * count, binary, start + i * stride)
        if accessor.get("normalized") and code in "BH":
            row = tuple(v / (255 if code == "B" else 65535) for v in row)
        rows.append(row)
    return rows


def convert(name, whale):
    document, binary = read_glb(SOURCE / whale["file"])
    positions, normals, uvs, indices = [], [], [], []
    for mesh in document["meshes"]:
        if mesh.get("name") not in whale["meshes"]:
            continue
        for primitive in mesh["primitives"]:
            offset = len(positions)
            attributes = primitive["attributes"]
            positions += read_accessor(document, binary, attributes["POSITION"])
            normals += read_accessor(document, binary, attributes["NORMAL"])
            uvs += read_accessor(document, binary, attributes["TEXCOORD_0"])
            indices += [offset + row[0] for row in read_accessor(document, binary, primitive["indices"])]

    def turn(vector):
        return tuple(vector[axis] * sign for axis, sign in whale["axes"])

    positions = [turn(p) for p in positions]
    normals = [turn(n) for n in normals]
    low = [min(p[i] for p in positions) for i in range(3)]
    high = [max(p[i] for p in positions) for i in range(3)]
    length = high[0] - low[0]
    # Centre along the body and across it; vertically, on the middle of the trunk (ignoring fins).
    trunk = sorted(p[1] for p in positions if abs(p[0] - (low[0] + high[0]) / 2) < length * 0.15)
    centre = ((low[0] + high[0]) / 2, (trunk[0] + trunk[-1]) / 2, (low[2] + high[2]) / 2)
    positions = [tuple((p[i] - centre[i]) / length for i in range(3)) for p in positions]

    OUTPUT.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT / f"{name}.mesh", "wb") as out:
        out.write(b"EWM1" + struct.pack("<II", len(positions), len(indices)))
        for p, n, uv in zip(positions, normals, uvs):
            out.write(struct.pack("<8f", *p, *n, uv[0], uv[1]))
        out.write(struct.pack(f"<{len(indices)}I", *indices))

    image = document["images"][whale["image"]]
    view = document["bufferViews"][image["bufferView"]]
    with tempfile.NamedTemporaryFile(suffix=".png") as original:
        original.write(binary[view.get("byteOffset", 0):view.get("byteOffset", 0) + view["byteLength"]])
        original.flush()
        subprocess.run(
            ["sips", "-Z", str(TEXTURE_SIZE), "-s", "format", "jpeg", "-s", "formatOptions", "85",
             original.name, "--out", str(OUTPUT / f"{name}.jpg")],
            check=True, stdout=subprocess.DEVNULL)

    extras = document["asset"].get("extras", {})
    print(f"{name}: {len(positions)} vertices, {len(indices) // 3} triangles, "
          f"height {high[1] - low[1]:.2f} and width {high[2] - low[2]:.2f} of length {length:.2f}")
    print(f"  from: {extras.get('title')}, {extras.get('author')}, {extras.get('license')}")
    print(f"  {extras.get('source')}")


if __name__ == "__main__":
    for whale_name, whale_config in WHALES.items():
        convert(whale_name, whale_config)
