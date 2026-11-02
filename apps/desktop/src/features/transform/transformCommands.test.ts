import { expect, it } from "vitest";
import { Matrix4 } from "three";
import { worldFrame } from "./frames";
import {
  beginTransform,
  setTransformAxis,
  setTransformInput,
} from "./transformSession";
import { prepareTransformCommands } from "./transformCommands";
it("builds one native batch with row-major offsets, preserving the captured source", () => {
  const capture = { projectId: "p", revisionId: "r", sceneToken: "s" };
  const target = (bodyId: string) => ({
    bodyId,
    originMm: [0, 0, 0] as [number, number, number],
    boundsMm: {
      min: [-1, -1, -1] as [number, number, number],
      max: [1, 1, 1] as [number, number, number],
    },
    matrixWorld: new Matrix4().toArray(),
  });
  const session = setTransformInput(
    setTransformAxis(
      beginTransform({
        capture,
        kind: "move",
        targets: [target("a"), target("b")],
        activeId: "b",
        pivotMode: "median",
        cursorMm: [0, 0, 0],
        orientation: worldFrame,
        local: worldFrame,
      })!,
      "x",
    ),
    "10mm",
  );
  const result = prepareTransformCommands(
    session,
    capture,
    ["move-a", "move-b"],
    "Move",
  );
  expect(result.session.phase).toBe("committing");
  expect(result.commands).toHaveLength(2);
  expect(result.commands[0]).toEqual({
    command: "transform_body",
    bodyId: "a",
    featureId: "move-a",
    name: "Move",
    matrix: [1, 0, 0, 10, 0, 1, 0, 0, 0, 0, 1, 0],
  });
  expect(
    prepareTransformCommands(
      result.session,
      capture,
      ["another-a", "another-b"],
      "Move",
    ).commands,
  ).toEqual([]);
  expect(
    prepareTransformCommands(
      session,
      capture,
      ["duplicate", "duplicate"],
      "Move",
    ).error,
  ).toBe("TRANSFORM_INPUT_INVALID");
  expect(
    prepareTransformCommands(
      session,
      { ...capture, revisionId: "other" },
      ["a", "b"],
      "Move",
    ).error,
  ).toBe("TRANSFORM_STALE");
});
