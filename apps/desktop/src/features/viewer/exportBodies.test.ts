import * as THREE from "three";
import { describe, expect, it } from "vitest";
import { exportMesh } from "../../lib/files";
import { exportBodies } from "./exportBodies";

function blobText(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsText(blob);
  });
}

describe("named body export", () => {
  it("reads stable IDs from a saved CAD IR revision", () => {
    const bodies = exportBodies(
      JSON.stringify({
        schemaVersion: 2,
        bodies: [
          { id: "housing", name: "Корпус", sourceFeatureId: "pad" },
          { id: "lid", name: "Крышка", sourceFeatureId: "lid_pad" },
        ],
      }),
    );
    expect(bodies).toEqual([
      { id: "housing", name: "Корпус" },
      { id: "lid", name: "Крышка" },
    ]);
  });

  it("exports only the selected mesh body", async () => {
    const group = new THREE.Group();
    for (const id of ["housing", "lid"]) {
      const mesh = new THREE.Mesh(new THREE.BoxGeometry(10, 10, 10));
      mesh.name = id;
      mesh.userData.formaBodyId = id;
      group.add(mesh);
    }
    const whole = await exportMesh(group, "obj");
    const selected = await exportMesh(group, "obj", "lid");
    expect((await blobText(selected)).match(/^v /gm)).toHaveLength(24);
    expect((await blobText(whole)).match(/^v /gm)).toHaveLength(48);
    await expect(exportMesh(group, "obj", "missing")).rejects.toThrow();
  });

  it("does not bake a selected-face overlay into the model export", async () => {
    const group = new THREE.Group();
    const body = new THREE.Mesh(new THREE.BoxGeometry(10, 10, 10));
    body.name = "housing";
    body.userData.formaBodyId = "housing";
    const overlay = new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1));
    overlay.userData.formaSelectionOverlay = true;
    body.add(overlay);
    group.add(body);
    const whole = await blobText(await exportMesh(group, "obj"));
    const selected = await blobText(await exportMesh(group, "obj", "housing"));
    expect(whole.match(/^v /gm)).toHaveLength(24);
    expect(selected.match(/^v /gm)).toHaveLength(24);
    expect(overlay.parent).toBe(body);
  });
});
