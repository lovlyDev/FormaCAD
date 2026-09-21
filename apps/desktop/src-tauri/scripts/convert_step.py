"""Trusted STEP tessellator. Reads only explicit local paths from the host."""
import json
import struct
import sys
from pathlib import Path
import cadquery as cq


def convert(source, output, parts=None):
    shape = cq.importers.importStep(str(source)).val()
    if not shape.isValid() or not shape.Solids():
        raise ValueError("STEP does not contain valid solids")
    components = parts or [{"name": f"Body {i+1}", "shape": solid, "pivot": [0,0,0], "axis": [0,0,1], "speed": 0} for i, solid in enumerate(shape.Solids())]
    document = {"asset": {"version": "2.0", "generator": "Forma CAD"},
                "scene": 0, "scenes": [{"nodes": []}], "nodes": [], "meshes": [],
                "buffers": [], "bufferViews": [], "accessors": []}
    binary = bytearray()
    vertex_count = 0
    for component in components:
        solid = component.get("shape")
        if solid is None:
            solid = cq.importers.importStep(component["source"]).val()
        vertices, triangles = solid.tessellate(0.1, 0.1)
        vertex_count += len(vertices)
        if not vertices or not triangles or vertex_count > 2_000_000:
            raise ValueError("Tessellation is empty or exceeds the preview limit")
        px, py, pz = component["pivot"]
        positions = [((v.x-px)/1000, (v.z-pz)/1000, -(v.y-py)/1000) for v in vertices]
        pos_offset = len(binary)
        binary.extend(b"".join(struct.pack("<3f", *v) for v in positions))
        idx_offset = len(binary)
        binary.extend(b"".join(struct.pack("<3I", *t) for t in triangles))
        view = len(document["bufferViews"])
        document["bufferViews"].extend([
            {"buffer":0,"byteOffset":pos_offset,"byteLength":idx_offset-pos_offset,"target":34962},
            {"buffer":0,"byteOffset":idx_offset,"byteLength":len(binary)-idx_offset,"target":34963}])
        accessor = len(document["accessors"])
        document["accessors"].extend([
            {"bufferView":view,"componentType":5126,"count":len(positions),"type":"VEC3",
             "min":[min(v[i] for v in positions) for i in range(3)],
             "max":[max(v[i] for v in positions) for i in range(3)]},
            {"bufferView":view+1,"componentType":5125,"count":len(triangles)*3,"type":"SCALAR"}])
        index = len(document["nodes"])
        ax, ay, az = component["axis"]
        document["nodes"].append({"mesh":index,"name":component["name"],"translation":[px/1000,pz/1000,-py/1000],
                                  "extras":{"formaMotion":{"axis":[ax,az,-ay],"speed":component["speed"]}}})
        document["meshes"].append({"primitives":[{"attributes":{"POSITION":accessor},"indices":accessor+1}]})
        document["scenes"][0]["nodes"].append(index)
    document["buffers"] = [{"byteLength":len(binary)}]
    raw = json.dumps(document, separators=(",", ":")).encode()
    raw += b" " * (-len(raw) % 4)
    data = struct.pack("<III", 0x46546C67, 2, 28+len(raw)+len(binary))
    data += struct.pack("<II", len(raw), 0x4E4F534A) + raw
    data += struct.pack("<II", len(binary), 0x004E4942) + binary
    Path(output).write_bytes(data)
    bounds = shape.BoundingBox()
    return {"solids": len(shape.Solids()), "volumeMm3": shape.Volume(),
            "areaMm2": shape.Area(), "boundsMm": [bounds.xlen, bounds.ylen, bounds.zlen]}


if __name__ == "__main__":
    request = json.load(sys.stdin.buffer)
    parts = json.loads(Path("parts.json").read_text(encoding="utf-8")) if request.get("parts") else None
    print(json.dumps(convert(request["source"], request["output"], parts)))
