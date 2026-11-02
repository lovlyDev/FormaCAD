import { describe, expect, it } from "vitest";
import { beginTransform } from "../transform/transformSession";
import { worldFrame } from "../transform/frames";
import { snapSessionIncrements } from "./incrementSnap";

const session = beginTransform({
  capture: { projectId: "p", revisionId: "r", sceneToken: "s" },
  kind: "move",
  targets: [
    {
      bodyId: "body",
      originMm: [0, 0, 0],
      boundsMm: { min: [-1, -1, -1], max: [1, 1, 1] },
      matrixWorld: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
    },
  ],
  activeId: "body",
  pivotMode: "bounds",
  cursorMm: [0, 0, 0],
  orientation: worldFrame,
  local: worldFrame,
})!;

describe("increment snapping", () => {
  it("rounds motion only when enabled", () => {
    const moved = {
      ...session,
      accumulated: {
        translationMm: [9.6, -0.4, 2.51] as [number, number, number],
        angleDeg: 12.6,
        scaleDelta: 0.44,
      },
    };
    expect(snapSessionIncrements(moved, false)).toBe(moved);
    const snapped = snapSessionIncrements(moved, true).accumulated;
    expect(snapped.translationMm).toEqual([10, 0, 3]);
    expect(snapped.angleDeg).toBe(15);
    expect(snapped.scaleDelta).toBeCloseTo(0.4);
  });
  it("never snaps scale to zero", () => {
    const shrunk = {
      ...session,
      accumulated: { ...session.accumulated, scaleDelta: -0.99 },
    };
    expect(
      1 + snapSessionIncrements(shrunk, true).accumulated.scaleDelta,
    ).toBeCloseTo(0.1);
  });
});
