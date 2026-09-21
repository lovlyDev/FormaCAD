"""Trusted, fixed CAD adapter. Input is data; agent-authored code is never evaluated."""
import json
import math
import sys
from pathlib import Path

import cadquery as cq


def build(p):
    w, d, h, t = (p[k] for k in ("width", "depth", "height", "thickness"))
    diameter, holes = p["holeDiameter"], p["holes"]
    assert all(math.isfinite(v) and 1 <= v <= 2000 for v in (w, d, h))
    assert .2 <= t <= 100 and t * 2 < min(w, d, h)
    kind = p["kind"]
    limit = min(w, d) / 4
    if kind == "box":
        columns = math.ceil(holes / 2)
        limit = min(w, d)*.9 if holes <= 1 else min(w*.36, d*.44, w*.64/(columns-1)*.9 if columns>1 else d*.5)
    assert 0 <= diameter <= min(200, limit) and 0 <= holes <= 16
    if kind == "box":
        model = cq.Workplane("XY").box(w, d, h, centered=(True, True, False))
        if diameter > 0 and holes:
            columns = math.ceil(holes / 2)
            points = [(0,0)] if holes == 1 else [(0 if columns==1 else -w*.32+(i%columns)*w*.64/(columns-1), d*.28 if i<columns else -d*.28) for i in range(holes)]
            model = model.faces(">Z").workplane().pushPoints(points).hole(diameter)
        return model
    if kind == "cylinder":
        profile = cq.Workplane("XY").circle(w / 2)
        if diameter > 0:
            profile = profile.circle(diameter / 2)
        return profile.extrude(h)
    model = cq.Workplane("XY").box(w, d, t, centered=(True, True, False))
    if kind in ("bracket", "plate") and diameter > 0 and holes:
        columns = math.ceil(holes / 2)
        # Viewer rotates its XY profile -90 degrees around X: profile Y maps to -Z.
        points = [(0 if columns == 1 else -w * .32 + (i % columns) * w * .64 / (columns - 1),
                   d * .28 if i < columns else -d * .28) for i in range(holes)]
        model = model.faces(">Z").workplane().pushPoints(points).hole(diameter)
    if kind == "bracket":
        wall = cq.Workplane("XY").box(w, t, h - t, centered=(True, True, False)).translate((0, -d / 2 + t / 2, t))
        model = model.union(wall)
    elif kind == "enclosure":
        outer = cq.Workplane("XY").box(w, d, h, centered=(True, True, False))
        inner = cq.Workplane("XY").box(w - 2*t, d - 2*t, h, centered=(True, True, False)).translate((0, 0, t))
        model = outer.cut(inner)
    elif kind != "plate":
        raise ValueError("Unsupported model kind")
    return model


def main():
    request = json.load(sys.stdin)
    output = Path(request["output"])
    if output.name != str(output) or output.suffix != ".step":
        raise ValueError("Only a STEP filename within the working directory is allowed")
    model = build(request["parameters"])
    shape = model.val()
    if not shape.isValid() or shape.Volume() <= 0:
        raise ValueError("Generated solid is invalid")
    cq.exporters.export(model, str(output))
    # Round-trip through the kernel before publishing success.
    loaded = cq.importers.importStep(str(output))
    if not loaded.val().isValid() or loaded.val().Volume() <= 0:
        raise ValueError("STEP round-trip validation failed")
    print(json.dumps({"type": "cad_exported", "file": output.name, "volume": shape.Volume()}))


if __name__ == "__main__":
    main()
