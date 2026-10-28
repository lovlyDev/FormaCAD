import { describe, expect, it } from "vitest";
import { Matrix4 } from "three";
import { worldFrame } from "../transform/frames";
import {
  beginTransform,
  setTransformAxis,
  setTransformInput,
} from "../transform/transformSession";
import {
  prepareAmendTransform,
  rememberLastTransform,
  reopenLastTransform,
} from "./amendTransform";
const baseCapture = {
  projectId: "p",
  revisionId: "before",
  sceneToken: "before-scene",
};
const head = {
  projectId: "p",
  revisionId: "after",
  sceneToken: "after-scene",
  transactionId: "transaction",
  generation: 4,
};
function fixture() {
  const base = setTransformInput(
    setTransformAxis(
      beginTransform({
        capture: baseCapture,
        kind: "move",
        targets: [
          {
            bodyId: "body",
            originMm: [0, 0, 0],
            boundsMm: { min: [-1, -1, -1], max: [1, 1, 1] },
            matrixWorld: new Matrix4().toArray(),
          },
        ],
        activeId: "body",
        pivotMode: "bounds",
        cursorMm: [0, 0, 0],
        orientation: worldFrame,
        local: worldFrame,
      })!,
      "x",
    ),
    "10mm",
  );
  return rememberLastTransform(base, head, ["move"], [0], "Move")!;
}
describe("F9 transform preparation", () => {
  it("replaces arguments relative to the original snapshot, not the already moved body", () => {
    const record = fixture(),
      reopened = reopenLastTransform(record, head)!;
    const updated = setTransformInput(reopened, "20mm");
    const result = prepareAmendTransform(record, updated, head);
    expect(result.plan).toEqual({
      expectedRevision: "after",
      expectedGeneration: 4,
      transactionId: "transaction",
      baseRevision: "before",
      replacements: [
        {
          commandIndex: 0,
          featureId: "move",
          matrix: [1, 0, 0, 20, 0, 1, 0, 0, 0, 0, 1, 0],
        },
      ],
    });
    expect(record.base.input).toBe("10mm");
    expect(record.base.targets[0].matrixWorld).toEqual(new Matrix4().toArray());
    // Replacing a previous 10 mm move by zero restores its original input; it is not a new no-op.
    expect(
      prepareAmendTransform(record, setTransformInput(reopened, "0mm"), head)
        .plan?.replacements[0].matrix[3],
    ).toBe(0);
  });
  it("rejects subsequent commits, history movement, another project and stale scenes", () => {
    const record = fixture(),
      session = reopenLastTransform(record, head)!;
    for (const changed of [
      { revisionId: "later" },
      { generation: 5 },
      { transactionId: "other" },
      { projectId: "other" },
    ]) {
      expect(reopenLastTransform(record, { ...head, ...changed })).toBeNull();
      expect(
        prepareAmendTransform(record, session, { ...head, ...changed }).error,
      ).toBe("AMEND_STALE");
    }
    expect(
      prepareAmendTransform(record, session, { ...head, sceneToken: "rebuilt" })
        .error,
    ).toBe("AMEND_STALE");
  });
});
