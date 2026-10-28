import { describe, expect, it } from "vitest";
import { Matrix4, Vector3 } from "three";
import { frameFromMatrix, normalFrame, validFrame, worldFrame } from "./frames";
import { selectionPivot } from "./pivots";
import {
  beginTransform,
  cancelTransform,
  confirmTransform,
  moveTransformPointer,
  setTransformAxis,
  setTransformInput,
  transformIntents,
} from "./transformSession";
import type { TransformSession, TransformTarget } from "./transformSession";
const capture = {
  projectId: "project-a",
  revisionId: "revision-a",
  sceneToken: "scene-a",
};
function target(bodyId = "body-a", x = 0): TransformTarget {
  return {
    bodyId,
    originMm: [x, 0, 0],
    boundsMm: { min: [x - 1, -1, -1], max: [x + 1, 1, 1] },
    matrixWorld: new Matrix4().makeTranslation(x, 0, 0).toArray(),
  };
}
function start(
  kind: TransformSession["kind"] = "move",
  extra = {},
): TransformSession {
  return beginTransform({
    capture,
    kind,
    targets: [target()],
    activeId: "body-a",
    pivotMode: "bounds",
    cursorMm: [0, 0, 0],
    orientation: worldFrame,
    local: worldFrame,
    ...extra,
  })!;
}
describe("modal transform lifecycle", () => {
  it("G X 10mm emits one captured intent, retains the original and cannot confirm twice", () => {
    const original = target();
    const session = setTransformInput(
      setTransformAxis(start("move", { targets: [original] }), "x"),
      "10mm",
    );
    const result = confirmTransform(session, capture);
    expect(result.session.phase).toBe("committing");
    expect(result.intents).toHaveLength(1);
    expect(result.intents![0].intent).toEqual({
      kind: "move",
      offsetMm: [10, 0, 0],
    });
    expect(
      new Vector3()
        .setFromMatrixPosition(
          new Matrix4().fromArray(result.intents![0].matrixWorld),
        )
        .toArray(),
    ).toEqual([10, 0, 0]);
    expect(original.matrixWorld).toEqual(new Matrix4().toArray());
    expect(confirmTransform(result.session, capture).intents).toBeNull();
  });
  it("cancel/no-op emit no transaction and a changed project/revision/scene cannot commit", () => {
    expect(
      confirmTransform(cancelTransform(start()), capture).intents,
    ).toBeNull();
    expect(confirmTransform(start(), capture).session.phase).toBe("cancelled");
    const session = setTransformInput(setTransformAxis(start(), "x"), "1in");
    for (const changed of [
      { projectId: "project-b" },
      { revisionId: "revision-b" },
      { sceneToken: "scene-b" },
    ]) {
      const result = confirmTransform(session, { ...capture, ...changed });
      expect(result.error).toBe("TRANSFORM_STALE");
      expect(result.intents).toBeNull();
    }
    const result = confirmTransform(setTransformInput(session, "-"), capture);
    expect(result.error).toBe("TRANSFORM_INPUT_INVALID");
    expect(result.session.phase).toBe("preview");
  });
  it("cycles world/local/unconstrained and excludes the selected plane axis", () => {
    const local = frameFromMatrix(
      new Matrix4().makeRotationZ(Math.PI / 2).toArray(),
    )!;
    let session = setTransformInput(
      setTransformAxis(start("move", { local }), "x"),
      "10mm",
    );
    session = setTransformAxis(session, "x");
    expect(transformIntents(session)![0].intent).toMatchObject({
      offsetMm: [expect.closeTo(0, 10), 10, 0],
    });
    session = setTransformAxis(session, "x");
    expect(session.constraint).toBeNull();
    session = moveTransformPointer(
      setTransformAxis(setTransformInput(session, ""), "x", true),
      { translationMm: [10, 20, 30], angleDeg: 0, scaleDelta: 0 },
      false,
    );
    expect(transformIntents(session)![0].intent).toEqual({
      kind: "move",
      offsetMm: [0, 20, 30],
    });
  });
  it("integrates precision per segment without a Shift-toggle jump", () => {
    let session = moveTransformPointer(
      start(),
      { translationMm: [10, 0, 0], angleDeg: 0, scaleDelta: 0 },
      false,
    );
    session = moveTransformPointer(
      session,
      { translationMm: [20, 0, 0], angleDeg: 0, scaleDelta: 0 },
      true,
    );
    session = moveTransformPointer(
      session,
      { translationMm: [30, 0, 0], angleDeg: 0, scaleDelta: 0 },
      false,
    );
    expect(transformIntents(session)![0].intent).toMatchObject({
      offsetMm: [21, 0, 0],
    });
    expect(
      moveTransformPointer(
        session,
        { translationMm: [NaN, 0, 0], angleDeg: 0, scaleDelta: 0 },
        false,
      ),
    ).toBe(session);
  });
  it("rotates around an offset pivot and scales in an oriented frame", () => {
    const session = setTransformInput(
      start("rotate", {
        targets: [target("body-a", 10)],
        pivotMode: "cursor",
        cursorMm: [5, 0, 0],
      }),
      "90deg",
    );
    const point = new Vector3().setFromMatrixPosition(
      new Matrix4().fromArray(transformIntents(session)![0].matrixWorld),
    );
    expect(point.x).toBeCloseTo(5);
    expect(point.y).toBeCloseTo(5);
    const scale = setTransformInput(
      setTransformAxis(
        start("scale", {
          targets: [target("body-a", 10)],
          pivotMode: "cursor",
          cursorMm: [5, 0, 0],
        }),
        "x",
      ),
      "200%",
    );
    expect(
      new Vector3().setFromMatrixPosition(
        new Matrix4().fromArray(transformIntents(scale)![0].matrixWorld),
      ).x,
    ).toBeCloseTo(15);
    expect(transformIntents(setTransformInput(scale, "-1"))).toBeNull();
  });
  it("supports individual origins without mutating another body or the snapshots", () => {
    const bodies = [target("body-a", 2), target("body-b", 20)];
    const session = setTransformInput(
      start("rotate", { targets: bodies, pivotMode: "individual" }),
      "90",
    );
    const results = transformIntents(session)!;
    expect(
      results.map(
        (result) =>
          new Vector3().setFromMatrixPosition(
            new Matrix4().fromArray(result.matrixWorld),
          ).x,
      ),
    ).toEqual([2, 20]);
    bodies[0].originMm[0] = 999;
    expect(session.targets[0].originMm[0]).toBe(2);
  });
});
describe("pivot/frame geometry", () => {
  it("distinguishes bounding-box, median, active and cursor pivots", () => {
    const anchors = [target("a", 0), target("b", 10), target("c", 50)];
    expect(selectionPivot(anchors, "bounds", "b", [1, 2, 3])).toEqual([
      25, 0, 0,
    ]);
    expect(selectionPivot(anchors, "median", "b", [1, 2, 3])).toEqual([
      20, 0, 0,
    ]);
    expect(selectionPivot(anchors, "active", "b", [1, 2, 3])).toEqual([
      10, 0, 0,
    ]);
    expect(selectionPivot(anchors, "cursor", "b", [1, 2, 3])).toEqual([
      1, 2, 3,
    ]);
    expect(selectionPivot(anchors, "active", "missing", [1, 2, 3])).toBeNull();
  });
  it("builds stable orthonormal frames at parallel normals and rejects invalid ones", () => {
    expect(validFrame(normalFrame([1, 0, 0])!)).toBe(true);
    expect(
      validFrame(frameFromMatrix(new Matrix4().makeScale(2, 3, 4).toArray())!),
    ).toBe(true);
    expect(normalFrame([0, 0, 0])).toBeNull();
    expect(normalFrame([NaN, 0, 1])).toBeNull();
    expect(normalFrame([Number.MAX_VALUE, 0, 1])).toBeNull();
    expect(
      frameFromMatrix(
        new Matrix4().makeScale(Number.MAX_VALUE, 1, 1).toArray(),
      ),
    ).toBeNull();
    expect(
      frameFromMatrix(new Matrix4().makeScale(0, 1, 1).toArray()),
    ).toBeNull();
    expect(validFrame({ ...worldFrame, z: [0, 0, -1] })).toBe(false);
  });
});
