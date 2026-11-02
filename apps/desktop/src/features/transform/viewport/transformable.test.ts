import { describe, expect, it } from "vitest";
import * as THREE from "three";
import type { Project } from "../../../types";
import { transformableBodies } from "./transformable";

const program = JSON.stringify({
  schemaVersion: 2,
  revisionId: "r",
  parameters: [],
  features: [
    {
      id: "profile",
      name: "Profile",
      operation: {
        type: "rectangle",
        width: { kind: "literal", mm: 4 },
        depth: { kind: "literal", mm: 2 },
      },
    },
    {
      id: "pad",
      name: "Pad",
      operation: {
        type: "extrude",
        sketchId: "profile",
        distance: { kind: "literal", mm: 1 },
      },
    },
  ],
  bodies: [{ id: "body", name: "Body", sourceFeatureId: "pad" }],
});
const project = {
  currentRevision: "r",
  revisions: [{ id: "r", program }],
} as unknown as Project;

describe("transformable bodies", () => {
  it("accepts only native previews whose bodies exist in the revision document", () => {
    const root = new THREE.Group();
    expect(transformableBodies(root, project, ["body"])).toBe(false);
    root.userData.formaNativeCadPreview = true;
    expect(transformableBodies(root, project, ["body"])).toBe(true);
    expect(transformableBodies(root, project, ["Body 1"])).toBe(false);
    expect(
      transformableBodies(
        root,
        { ...project, currentRevision: "x" } as Project,
        ["body"],
      ),
    ).toBe(false);
  });
});
