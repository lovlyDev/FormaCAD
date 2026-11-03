import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { projectBodies } from "./projectBodies";
import { regionSelection } from "./regionSelection";
import { boxRegion } from "./regionGeometry";

function scene() {
  const group = new THREE.Group();
  const left = new THREE.Mesh(new THREE.BoxGeometry(10, 10, 10));
  left.name = "left";
  left.position.set(-50, 0, 0);
  const right = new THREE.Mesh(new THREE.BoxGeometry(10, 10, 10));
  right.name = "right";
  right.position.set(50, 0, 0);
  group.add(left, right);
  const camera = new THREE.OrthographicCamera(-100, 100, 75, -75, 0.1, 1000);
  camera.position.set(0, 0, 200);
  camera.lookAt(0, 0, 0);
  camera.updateMatrixWorld();
  return { group, camera, left, right };
}
const size = { width: 800, height: 600 };

describe("box selection projection", () => {
  it("selects bodies whose projected triangles meet the box", () => {
    const { group, camera } = scene();
    const projected = projectBodies(group, camera, size);
    expect(
      regionSelection(boxRegion([0, 0], [380, 600]), projected, "body", true),
    ).toEqual(["left"]);
    expect(
      regionSelection(
        boxRegion([0, 0], [800, 600]),
        projected,
        "body",
        true,
      ).sort(),
    ).toEqual(["left", "right"]);
  });
  it("selects a large face when the whole box lies inside it and skips hidden or locked bodies", () => {
    const { group, camera, left, right } = scene();
    expect(
      regionSelection(
        boxRegion([195, 295], [205, 305]),
        projectBodies(group, camera, size),
        "body",
        true,
      ),
    ).toEqual(["left"]);
    left.visible = false;
    right.userData.formaSelectionLocked = true;
    expect(
      regionSelection(
        boxRegion([0, 0], [800, 600]),
        projectBodies(group, camera, size),
        "body",
        true,
      ),
    ).toEqual([]);
  });
});
