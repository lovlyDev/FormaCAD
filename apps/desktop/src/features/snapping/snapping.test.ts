import { describe, expect, it } from "vitest";
import { snapGrid, snapIncrement } from "./gridSnap";
import { readExactSnap } from "./exactSnapSession";
const expected = {
  requestId: "00000000-0000-4000-8000-000000000001",
  projectId: "p",
  revisionId: "r",
  sceneToken: "s",
  sourceSha256: "a".repeat(64),
  documentSha256: "b".repeat(64),
  bodyId: "body",
};
const candidate = {
  kind: "circleCenter",
  reference: {
    schemaVersion: 1,
    ownerFeatureId: "pad",
    occurrencePath: [],
    kind: "edge",
    role: "cylinder-edge:top",
  },
  pointMm: [10, 20, 30],
};
const sample = { ...expected, engine: "occt", candidates: [candidate] };
describe("snap source ownership", () => {
  it("accepts only a bounded native result for the exact captured source and scene", () => {
    expect(
      readExactSnap(sample, expected, expected)?.candidates[0].pointMm,
    ).toEqual([10, 20, 30]);
    for (const changed of [
      { requestId: "00000000-0000-4000-8000-000000000002" },
      { sourceSha256: "c".repeat(64) },
      { documentSha256: "c".repeat(64) },
      { bodyId: "other" },
      { projectId: "other" },
    ])
      expect(
        readExactSnap({ ...sample, ...changed }, expected, expected),
      ).toBeNull();
    expect(
      readExactSnap(sample, expected, { ...expected, sceneToken: "new" }),
    ).toBeNull();
    expect(
      readExactSnap({ ...sample, engine: "mesh" }, expected, expected),
    ).toBeNull();
    expect(
      readExactSnap(
        { ...sample, candidates: [{ ...candidate, pointMm: [NaN, 0, 0] }] },
        expected,
        expected,
      ),
    ).toBeNull();
  });
  it("requires face normals and rejects mismatched topology references", () => {
    expect(
      readExactSnap(
        { ...sample, candidates: [{ ...candidate, kind: "face" }] },
        expected,
        expected,
      ),
    ).toBeNull();
    expect(
      readExactSnap(
        { ...sample, candidates: Array(129).fill(candidate) },
        expected,
        expected,
      ),
    ).toBeNull();
  });
  it("snaps relative to a chosen origin and keeps step arithmetic finite", () => {
    expect(snapGrid([10.04, -1.96, 3], [0.05, 0.05, 0], 0.1)).toEqual([
      10.05, -1.95, 3,
    ]);
    expect(snapIncrement(0.30000000000000004, 0.1)).toBe(0.3);
    expect(snapIncrement(5, 0)).toBeNull();
    expect(snapIncrement(Infinity, 1)).toBeNull();
  });
});
