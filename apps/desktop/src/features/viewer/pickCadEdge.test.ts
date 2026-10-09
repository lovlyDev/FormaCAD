import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { pickCadEdge } from "./pickCadEdge";
describe("CAD edge screen picking", () => {
  function setup() {
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(2, 2, 2)); mesh.name = "body";
    mesh.geometry.userData.formaEdges = [
      { lengthMm: 2, points: [[-1, 1, 1], [1, 1, 1]] },
      { lengthMm: 2, points: [[0, -1, 1.1], [0, 1, 1.1]] },
      { lengthMm: 2, points: [[-1, 0, -1], [1, 0, -1]] },
    ];
    const root = new THREE.Group(); root.add(mesh);
    const camera = new THREE.OrthographicCamera(-2, 2, 2, -2, .1, 100); camera.position.z = 10; camera.updateProjectionMatrix();
    return { root, camera };
  }
  it("chooses the closest cursor segment instead of the nearer depth", () => {
    const { root, camera } = setup();
    expect(pickCadEdge(root, camera, { x: 215, y: 101 }, 400, 400)?.edgeOrdinal).toBe(1);
    expect(pickCadEdge(root, camera, { x: 201, y: 180 }, 400, 400)?.edgeOrdinal).toBe(2);
  });
  it("rejects clicks outside eight pixels and edges hidden behind the solid", () => {
    const { root, camera } = setup();
    expect(pickCadEdge(root, camera, { x: 250, y: 120 }, 400, 400)).toBeNull();
    expect(pickCadEdge(root, camera, { x: 250, y: 200 }, 400, 400)).toBeNull();
    camera.zoom = 2; camera.updateProjectionMatrix();
    expect(pickCadEdge(root, camera, { x: 270, y: 20 }, 400, 400)).toBeNull();
  });
});
