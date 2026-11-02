import { describe, expect, it } from "vitest";
import {
  beginTransform,
  setTransformAxis,
  setTransformInput,
  transformIntents,
} from "../transform/transformSession";
import { worldFrame } from "../transform/frames";
import { applyVertexSnap } from "./vertexSnap";

const session = beginTransform({
  capture: { projectId: "p", revisionId: "r", sceneToken: "s" },
  kind: "move",
  targets: [
    {
      bodyId: "body",
      originMm: [0, 0, 5],
      boundsMm: { min: [-1, -1, 0], max: [1, 1, 10] },
      matrixWorld: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
    },
  ],
  activeId: "body",
  pivotMode: "bounds",
  cursorMm: [0, 0, 0],
  orientation: worldFrame,
  local: worldFrame,
})!;

describe("vertex snapping during move", () => {
  it("puts the pivot exactly on the target", () => {
    const snapped = applyVertexSnap(session, [20, -10, 10]);
    expect(transformIntents(snapped)![0].intent).toEqual({
      kind: "move",
      offsetMm: [20, -10, 5],
    });
  });
  it("respects axis constraints and never overrides a typed value", () => {
    const along = applyVertexSnap(
      setTransformAxis(session, "x"),
      [20, -10, 10],
    );
    expect(transformIntents(along)![0].intent).toEqual({
      kind: "move",
      offsetMm: [20, 0, 0],
    });
    const typed = setTransformInput(session, "3");
    expect(applyVertexSnap(typed, [20, -10, 10])).toBe(typed);
    expect(applyVertexSnap(session, null)).toBe(session);
  });
});
