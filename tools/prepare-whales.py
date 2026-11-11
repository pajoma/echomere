#!/usr/bin/env python3
"""Prepares the whale models for the app.

Reads the original GLB downloads from assets/whales/source/ (not in the repository; see
assets/whales/README.md for where to get them) and writes, for each whale, a compact mesh and a reduced
texture to macos/EchomereScene/Sources/CoveSceneRenderer/Resources/Whales/.

The mesh keeps the model's bind pose and its skeleton weights. The model's own swim animation is baked into
an animation file: for every frame and every joint, the matrix that takes a bind-pose vertex to its place in
the swimming whale, already turned so that x runs from tail (-0.5) to snout (+0.5), y is up and z is to the
whale's side, and scaled so the whale is one unit long.

Mesh file (little-endian): "EWM2", vertex count (uint32), index count (uint32); then per vertex eight
float32 (position x y z, normal x y z, texture u v), four uint16 (joints) and four float32 (weights); then
the indices as uint32.

Animation file: "EWA1", joint count (uint32), frame count (uint32), duration in seconds (float32); then
for every frame, for every joint, twelve float32: the first three rows of its 4x4 matrix. The animation
loops: the frame after the last is the first.

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
FRAMES_PER_SECOND = 15

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
COUNTS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


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


# 4x4 matrices as lists of four rows.

def multiply(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def from_columns(values):
    """glTF stores matrices column by column."""
    return [[values[column * 4 + row] for column in range(4)] for row in range(4)]


def compose(translation, rotation, scale):
    x, y, z, w = rotation
    r = [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
         [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
         [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]
    return [[r[i][0] * scale[0], r[i][1] * scale[1], r[i][2] * scale[2], translation[i]] for i in range(3)] \
        + [[0.0, 0.0, 0.0, 1.0]]


def inverse_affine(m):
    """Inverse of a matrix whose last row is 0 0 0 1."""
    a = [row[:3] for row in m[:3]]
    det = (a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1]) - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
           + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]))
    inv = [[(a[(j + 1) % 3][(i + 1) % 3] * a[(j + 2) % 3][(i + 2) % 3]
             - a[(j + 1) % 3][(i + 2) % 3] * a[(j + 2) % 3][(i + 1) % 3]) / det for j in range(3)] for i in range(3)]
    t = [m[i][3] for i in range(3)]
    return [inv[i] + [-sum(inv[i][k] * t[k] for k in range(3))] for i in range(3)] + [[0.0, 0.0, 0.0, 1.0]]


def sample(times, values, t, is_rotation):
    """The value of an animation channel at time t (linear interpolation)."""
    if t <= times[0] or len(times) == 1:
        return values[0]
    if t >= times[-1]:
        return values[-1]
    low, high = 0, len(times) - 1
    while high - low > 1:
        middle = (low + high) // 2
        if times[middle] <= t:
            low = middle
        else:
            high = middle
    f = (t - times[low]) / (times[high] - times[low])
    a, b = values[low], values[high]
    if is_rotation and sum(x * y for x, y in zip(a, b)) < 0:
        b = tuple(-x for x in b)
    mixed = [x + (y - x) * f for x, y in zip(a, b)]
    if is_rotation:
        norm = sum(x * x for x in mixed) ** 0.5
        mixed = [x / norm for x in mixed]
    return mixed


class Skeleton:
    """The model's nodes, its skin and its first animation."""

    def __init__(self, document, binary):
        self.nodes = document["nodes"]
        self.parent = {}
        for index, node in enumerate(self.nodes):
            for child in node.get("children", []):
                self.parent[child] = index
        skin = document["skins"][0]
        self.joints = skin["joints"]
        self.inverse_bind = [from_columns(row) for row in read_accessor(document, binary, skin["inverseBindMatrices"])]
        animation = document["animations"][0]
        self.channels = []
        self.start, self.end = float("inf"), float("-inf")
        for channel in animation["channels"]:
            sampler = animation["samplers"][channel["sampler"]]
            times = [row[0] for row in read_accessor(document, binary, sampler["input"])]
            values = read_accessor(document, binary, sampler["output"])
            self.channels.append((channel["target"]["node"], channel["target"]["path"], times, values))
            if len(times) > 1:
                self.start, self.end = min(self.start, times[0]), max(self.end, times[-1])

    def local(self, index, animated):
        node = self.nodes[index]
        if "matrix" in node and index not in animated:
            return from_columns(node["matrix"])
        pose = animated.get(index, {})
        return compose(pose.get("translation", node.get("translation", (0, 0, 0))),
                       pose.get("rotation", node.get("rotation", (0, 0, 0, 1))),
                       pose.get("scale", node.get("scale", (1, 1, 1))))

    def skin_matrices(self, t):
        """For every joint, the matrix from the bind pose to the pose at time t (None: the rest pose)."""
        animated = {}
        if t is not None:
            for node, path, times, values in self.channels:
                animated.setdefault(node, {})[path] = sample(times, values, t, path == "rotation")
        world = {}

        def world_of(index):
            if index not in world:
                local = self.local(index, animated)
                world[index] = multiply(world_of(self.parent[index]), local) if index in self.parent else local
            return world[index]

        return [multiply(world_of(joint), inverse) for joint, inverse in zip(self.joints, self.inverse_bind)]


def convert(name, whale):
    document, binary = read_glb(SOURCE / whale["file"])
    positions, normals, uvs, indices, joints, weights = [], [], [], [], [], []
    for mesh in document["meshes"]:
        if mesh.get("name") not in whale["meshes"]:
            continue
        for primitive in mesh["primitives"]:
            offset = len(positions)
            attributes = primitive["attributes"]
            positions += read_accessor(document, binary, attributes["POSITION"])
            normals += read_accessor(document, binary, attributes["NORMAL"])
            uvs += read_accessor(document, binary, attributes["TEXCOORD_0"])
            joints += read_accessor(document, binary, attributes["JOINTS_0"])
            weights += read_accessor(document, binary, attributes["WEIGHTS_0"])
            indices += [offset + row[0] for row in read_accessor(document, binary, primitive["indices"])]

    def turn(vector):
        return tuple(vector[axis] * sign for axis, sign in whale["axes"])

    turned = [turn(p) for p in positions]
    low = [min(p[i] for p in turned) for i in range(3)]
    high = [max(p[i] for p in turned) for i in range(3)]
    length = high[0] - low[0]
    # Centre along the body and across it; vertically, on the middle of the trunk (ignoring fins).
    trunk = sorted(p[1] for p in turned if abs(p[0] - (low[0] + high[0]) / 2) < length * 0.15)
    centre = ((low[0] + high[0]) / 2, (trunk[0] + trunk[-1]) / 2, (low[2] + high[2]) / 2)
    # From the bind pose to our whale: turn, centre, and scale to a length of one.
    to_whale = [[(sign / length if column == axis else 0.0) for column in range(3)] + [-centre[row] / length]
                for row, (axis, sign) in enumerate(whale["axes"])] + [[0.0, 0.0, 0.0, 1.0]]

    OUTPUT.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT / f"{name}.mesh", "wb") as out:
        out.write(b"EWM2" + struct.pack("<II", len(positions), len(indices)))
        for p, n, uv, j, w in zip(positions, normals, uvs, joints, weights):
            total = sum(w) or 1.0
            out.write(struct.pack("<8f4H4f", *p, *n, uv[0], uv[1], *j, *(x / total for x in w)))
        out.write(struct.pack(f"<{len(indices)}I", *indices))

    # The swim animation. The skeleton's root joint doesn't swim: its matrix is the model's own placement in
    # its file. Undo that, so only the swimming movement is left. (The file's default pose can't be used for
    # this: in rigs driven by control nodes, such as the orca's, it is not the bind pose.)
    skeleton = Skeleton(document, binary)
    duration = skeleton.end - skeleton.start
    frames = round(duration * FRAMES_PER_SECOND)
    poses = [skeleton.skin_matrices(skeleton.start + duration * frame / frames) for frame in range(frames)]
    root = [[sum(pose[0][r][c] for pose in poses) / frames for c in range(4)] for r in range(4)]
    root_moves = max(abs(pose[0][r][c] - root[r][c]) for pose in poses for r in range(3) for c in range(4))
    from_file = multiply(to_whale, inverse_affine(root))
    with open(OUTPUT / f"{name}.animation", "wb") as out:
        out.write(b"EWA1" + struct.pack("<IIf", len(skeleton.joints), frames, duration))
        for pose in poses:
            for matrix in pose:
                placed = multiply(from_file, matrix)
                out.write(struct.pack("<12f", *placed[0], *placed[1], *placed[2]))

    # Where the animation puts the whale, to check its placement: the box around every joint's origin.
    origins = [multiply(multiply(from_file, matrix), inverse_affine(inverse))
               for pose in poses for matrix, inverse in zip(pose, skeleton.inverse_bind)]
    box = [(min(m[i][3] for m in origins), max(m[i][3] for m in origins)) for i in range(3)]

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
    print(f"{name}: {len(positions)} vertices, {len(indices) // 3} triangles, {len(skeleton.joints)} joints, "
          f"{frames} frames over {duration:.2f} s")
    print(f"  root joint moves by {root_moves:.4f}; joints stay within x {box[0][0]:.2f}..{box[0][1]:.2f}, "
          f"y {box[1][0]:.2f}..{box[1][1]:.2f}, z {box[2][0]:.2f}..{box[2][1]:.2f} (whale lengths)")
    print(f"  from: {extras.get('title')}, {extras.get('author')}, {extras.get('license')}")
    print(f"  {extras.get('source')}")


if __name__ == "__main__":
    for whale_name, whale_config in WHALES.items():
        convert(whale_name, whale_config)
