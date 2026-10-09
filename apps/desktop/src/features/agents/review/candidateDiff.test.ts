import { describe, expect, it } from "vitest";
import { newProject } from "../../../stores/workspace";
import { defaults } from "../../../types";
import { cadValuesEqual, candidateDiff } from "./candidateDiff";

const document = { schemaVersion: 2, revisionId: "base", parameters: [{ id: "width", name: "Width", valueMm: 40 }],
  features: [{ id: "profile", name: "Profile", operation: { type: "sketch2d", points: [{ id: "a", x: 1, y: 2 }] } },
    { id: "solid", name: "Solid", operation: { type: "extrude", sketchId: "profile", distance: { parameterId: "width" } } }],
  bodies: [{ id: "body", name: "Part", sourceFeatureId: "solid" }] };
function project() {
  const base = newProject("Review", "blank", "mm", "codex");
  base.currentRevision = "base";
  base.revisions = [{ id: "base", parent: null, createdAt: base.createdAt, prompt: "", parameters: defaults, program: JSON.stringify(document) }];
  return base;
}
describe("AI candidate comparison", () => {
  it("ignores serialization key order, but preserves ordered geometry and history", () => {
    expect(cadValuesEqual({ a: 1, b: { x: 2, y: [3, 4] } }, { b: { y: [3, 4], x: 2 }, a: 1 })).toBe(true);
    expect(cadValuesEqual([3, 4], [4, 3])).toBe(false);
    const after = { ...document, revisionId: "candidate", features: document.features.map(feature => ({ operation: feature.operation, name: feature.name, id: feature.id })) };
    const diff = candidateDiff(project(), JSON.stringify(after))!;
    expect([diff.parameters, diff.features, diff.bodies]).toEqual([[], [], []]);
  });
  it("reports parameters, coordinate bindings, removal and history order without changing the snapshot", () => {
    const base = project(), original = JSON.stringify(base), after = structuredClone(document);
    after.parameters[0].valueMm = 80;
    after.features.reverse();
    Object.assign(after.features[1].operation, { bindings: [{ id: "link", target: "a", axis: "x", value: { parameterId: "width" } }] });
    after.bodies = [];
    const diff = candidateDiff(base, JSON.stringify(after))!;
    expect(diff.parameters[0].before?.value.valueMm).toBe(40);
    expect(diff.parameters[0].after?.value.valueMm).toBe(80);
    expect(diff.features.map(change => [change.id, change.before?.index, change.after?.index])).toEqual([["profile", 0, 1], ["solid", 1, 0]]);
    expect(diff.bodies[0].after).toBeNull();
    expect(JSON.stringify(base)).toBe(original);
  });
  it("uses an empty baseline only for an empty project, never for imported/legacy geometry", () => {
    const base = newProject("Empty", "blank", "mm", "codex");
    expect(candidateDiff(base, JSON.stringify(document))?.features.every(change => change.before === null)).toBe(true);
    const imported = project(); imported.revisions[0].program = undefined;
    expect(candidateDiff(imported, JSON.stringify(document))).toBeNull();
    expect(candidateDiff(base, "invalid")).toBeNull();
  });
});
