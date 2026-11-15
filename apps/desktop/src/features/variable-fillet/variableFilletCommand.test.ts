import { expect, it } from "vitest";
import type { Project } from "../../types";
import type { EdgeSelection } from "../viewer/edgeSelection";
import { captureVariableFillet } from "./variableFilletCommand";
const selection: EdgeSelection = { bodyId: "body", revisionId: "rev", edgeOrdinal: 1,
  topologyRef: { schemaVersion: 1, kind: "edge", ownerFeatureId: "pad", role: "box-edge:z:xmax:ymax", occurrencePath: [] } };
function project(): Project { return { id: "project", currentRevision: "rev", revisions: [{ id: "rev", program: JSON.stringify({
  schemaVersion: 2, revisionId: "rev", parameters: [], features: [
    { id: "profile", name: "Profile", operation: { type: "rectangle", width: { kind: "literal", mm: 40 }, depth: { kind: "literal", mm: 20 } } },
    { id: "pad", name: "Pad", operation: { type: "extrude", sketchId: "profile", distance: { kind: "literal", mm: 10 } } },
  ], bodies: [{ id: "body", name: "Body", sourceFeatureId: "pad" }],
}) }] } as Project; }
it("binds distinct fractional radii to the captured edge without rewriting the source history", () => {
  const base = project(), before = JSON.stringify(base), edit = captureVariableFillet(base, [selection], 1.25, 3.5)!;
  const source = JSON.parse(edit.previewSource), old = JSON.parse(base.revisions[0].program!);
  expect(JSON.stringify(base)).toBe(before);
  expect(source.features.slice(0, 2)).toEqual(old.features);
  expect(source.features[2].operation).toEqual({ type: "variableFillet", bodyFeatureId: "pad", reference: selection.topologyRef,
    radiusStart: { kind: "literal", mm: 1.25 }, radiusEnd: { kind: "literal", mm: 3.5 } });
  expect(source.bodies[0].sourceFeatureId).toBe(source.features[2].id);
});
it("refuses ambiguous, stale and unsupported selections or invalid endpoint radii", () => {
  const base = project();
  for (const selected of [[], [selection, selection], [{ ...selection, revisionId: "old" }], [{ ...selection, topologyRef: undefined }]])
    expect(captureVariableFillet(base, selected, 1, 3)).toBeNull();
  for (const bad of [0, -1, NaN, Infinity, 10001, 1e-8]) {
    expect(captureVariableFillet(base, [selection], bad, 3)).toBeNull();
    expect(captureVariableFillet(base, [selection], 1, bad)).toBeNull();
  }
});
