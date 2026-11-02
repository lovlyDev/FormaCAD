import { describe, expect, it } from "vitest";
import { bodyLocalFrame } from "./bodyOrientation";
import { worldFrame } from "./frames";
import type { TypedCadDocument } from "../../lib/typedCadDocument";

const literal = (mm: number) => ({ kind: "literal" as const, mm });
function document(extra: object[], source: string) {
  return {
    schemaVersion: 2,
    revisionId: "r",
    parameters: [],
    features: [
      {
        id: "profile",
        name: "Profile",
        operation: { type: "rectangle", width: literal(4), depth: literal(2) },
      },
      {
        id: "pad",
        name: "Pad",
        operation: {
          type: "extrude",
          sketchId: "profile",
          distance: literal(1),
        },
      },
      ...extra,
    ],
    bodies: [{ id: "body", name: "Body", sourceFeatureId: source }],
  } as unknown as TypedCadDocument;
}
const near = (actual: number[], expected: number[]) =>
  expected.forEach((value, index) =>
    expect(actual[index]).toBeCloseTo(value, 9),
  );

describe("body local frame", () => {
  it("is the world frame for a constructed body", () => {
    expect(bodyLocalFrame(document([], "pad"), "body")).toEqual(worldFrame);
  });
  it("follows rotations and transform matrices but ignores translation and scale", () => {
    const frame = bodyLocalFrame(
      document(
        [
          {
            id: "spin",
            name: "Spin",
            operation: {
              type: "rotate",
              bodyFeatureId: "pad",
              axisOriginMm: [5, 0, 0],
              axisDirection: [0, 0, 1],
              angleDeg: 90,
            },
          },
          {
            id: "move",
            name: "Move",
            operation: {
              type: "transform",
              bodyFeatureId: "spin",
              matrix: [3, 0, 0, 10, 0, 3, 0, 0, 0, 0, 3, 0],
            },
          },
        ],
        "move",
      ),
      "body",
    );
    near(frame.x, [0, 1, 0]);
    near(frame.y, [-1, 0, 0]);
    near(frame.z, [0, 0, 1]);
  });
  it("stays right-handed after a mirror", () => {
    const frame = bodyLocalFrame(
      document(
        [
          {
            id: "flip",
            name: "Flip",
            operation: {
              type: "mirror",
              bodyFeatureId: "pad",
              planeOriginMm: [0, 0, 0],
              planeNormal: [1, 0, 0],
            },
          },
        ],
        "flip",
      ),
      "body",
    );
    near(frame.z, [0, 0, 1]);
    near(frame.x, [-1, 0, 0]);
    near(frame.y, [0, -1, 0]);
  });
});
