import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { visibleRegionHits } from "./occlusion";
import { boxRegion } from "./regionGeometry";

function scene() {
  const group = new THREE.Group();
  const front = new THREE.Mesh(new THREE.BoxGeometry(40, 40, 10));
  front.name = "front";
  front.position.set(0, 0, 20);
  const back = new THREE.Mesh(new THREE.BoxGeometry(20, 20, 10));
  back.name = "back";
  back.position.set(0, 0, -20);
  const side = new THREE.Mesh(new THREE.BoxGeometry(10, 10, 10));
  side.name = "side";
  side.position.set(60, 0, -20);
  group.add(front, back, side);
  group.updateMatrixWorld(true);
  const camera = new THREE.OrthographicCamera(-100, 100, 75, -75, 0.1, 1000);
  camera.position.set(0, 0, 200);
  camera.lookAt(0, 0, 0);
  camera.updateMatrixWorld();
  return { group, camera };
}
const size = { width: 800, height: 600 };

describe("region occlusion", () => {
  it("drops bodies hidden behind others and keeps visible ones", () => {
    const { group, camera } = scene();
    const region = boxRegion([300, 200], [700, 400]);
    expect(
      visibleRegionHits({
        object: group,
        camera,
        size,
        region,
        hits: ["front", "back", "side"],
      }).sort(),
    ).toEqual(["front", "side"]);
  });
});
