import { describe, expect, it } from "vitest";
import { addConstructionLine, removeConstructionLine } from "./sketchConstruction";
import { createStarterSketch, readSketchOperation } from "./sketchDocument";

function starter() {
  return readSketchOperation(JSON.parse(createStarterSketch()).features[0].operation)!;
}

describe("construction geometry", () => {
  it("round-trips guides without changing the profile or stable point IDs", () => {
    const original = starter();
    let changed = addConstructionLine(original, "point_a", "point_c")!;
    expect(original.lines).toHaveLength(4);
    expect(readSketchOperation(JSON.parse(JSON.stringify(changed)))?.lines).toHaveLength(5);
    expect(changed.points).toEqual(original.points);
    expect(changed.lines.at(-1)?.construction).toBe(true);
    changed = { ...changed, constraints: [...changed.constraints, { id: "diagonal_length", kind: "length", lineId: changed.lines.at(-1)!.id, distance: { kind: "literal", mm: 50 } }] };
    expect(removeConstructionLine(changed, changed.lines.at(-1)!.id)).toEqual(original);
    expect(removeConstructionLine(original, "line_ab")).toBe(original);
  });
  it("rejects missing/same endpoints and bounds the number of guides", () => {
    const original = starter();
    expect(addConstructionLine(original, "point_a", "point_a")).toBeNull();
    expect(addConstructionLine(original, "missing", "point_a")).toBeNull();
    let next = original;
    while (next.lines.length < 64) next = addConstructionLine(next, "point_a", "point_c")!;
    expect(new Set(next.lines.map((line) => line.id)).size).toBe(64);
    expect(addConstructionLine(next, "point_a", "point_c")).toBeNull();
    expect(readSketchOperation({ ...next, lines: [...next.lines, next.lines[0]] })).toBeNull();
  });
  it("removes orphan guide points and their constraints while keeping shared geometry", () => {
    const original = starter();
    const draft = { ...original, points: [...original.points, { id: "guide_point", xMm: 100, yMm: 100 }],
      lines: [...original.lines, { id: "guide", startPointId: "point_a", endPointId: "guide_point", construction: true }],
      constraints: [...original.constraints, { id: "anchor", kind: "fixed" as const, pointId: "guide_point", xMm: 100, yMm: 100 },
        { id: "coincide", kind: "coincident" as const, firstPointId: "guide_point", secondPointId: "point_c" }] };
    expect(removeConstructionLine(draft, "guide")).toEqual(original);
  });
});
