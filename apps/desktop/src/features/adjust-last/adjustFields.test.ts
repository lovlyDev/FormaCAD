import { describe, expect, it } from "vitest";
import {
  beginTransform,
  setTransformAxis,
  setTransformInput,
} from "../transform/transformSession";
import { worldFrame } from "../transform/frames";
import { prepareTransformCommands } from "../transform/transformCommands";
import { fieldsFromSession, sessionFromFields } from "./adjustFields";

const capture = { projectId: "p", revisionId: "r", sceneToken: "s" };
const base = beginTransform({
  capture,
  kind: "move",
  targets: [
    {
      bodyId: "body",
      originMm: [5, 0, 0],
      boundsMm: { min: [0, -1, -1], max: [10, 1, 1] },
      matrixWorld: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
    },
  ],
  activeId: "body",
  pivotMode: "bounds",
  cursorMm: [0, 0, 0],
  orientation: worldFrame,
  local: worldFrame,
})!;

describe("F9 parameter fields", () => {
  it("reads a typed G X 10mm and replaces it with 20 mm, not 30 mm", () => {
    const typed = setTransformInput(setTransformAxis(base, "x"), "10mm");
    expect(fieldsFromSession(typed)).toEqual({
      kind: "move",
      offsetMm: [10, 0, 0],
    });
    const amended = sessionFromFields(base, {
      kind: "move",
      offsetMm: [20, 0, 0],
    });
    const { commands } = prepareTransformCommands(
      amended,
      capture,
      ["t"],
      "Move",
      "amend",
    );
    expect(commands[0].matrix[3]).toBe(20);
  });
  it("round-trips rotation and single-axis scale", () => {
    const rotate = sessionFromFields(base, {
      kind: "rotate",
      axis: "y",
      angleDeg: 30,
    });
    expect(fieldsFromSession(rotate)).toEqual({
      kind: "rotate",
      axis: "y",
      angleDeg: 30,
    });
    const scale = sessionFromFields(base, {
      kind: "scale",
      axis: "x",
      factor: 2,
    });
    expect(fieldsFromSession(scale)).toEqual({
      kind: "scale",
      axis: "x",
      factor: 2,
    });
    const uniform = sessionFromFields(base, {
      kind: "scale",
      axis: null,
      factor: 0.5,
    });
    expect(fieldsFromSession(uniform)).toEqual({
      kind: "scale",
      axis: null,
      factor: 0.5,
    });
  });
});
