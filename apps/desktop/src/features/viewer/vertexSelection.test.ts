import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { cadVertices, pickCadVertex } from "./vertexSelection";

const reference = {
  schemaVersion: 1,
  kind: "vertex",
  ownerFeatureId: "pad",
  occurrencePath: [],
  role: "box-vertex:xmax:ymin:zmax",
};
function scene() {
  const group = new THREE.Group();
  group.scale.setScalar(1000);
  const mesh = new THREE.Mesh(new THREE.BoxGeometry(0.01, 0.01, 0.01));
  mesh.name = "body";
  mesh.geometry.userData.formaVertices = [
    { topologyRef: reference, point: [0.01, 0, 0] },
    { topologyRef: null, point: [-0.01, 0, 0] },
  ];
  group.add(mesh);
  const camera = new THREE.OrthographicCamera(-50, 50, 50, -50, 0.1, 1000);
  camera.position.set(0, 0, 100);
  camera.lookAt(0, 0, 0);
  camera.updateMatrixWorld();
  group.updateMatrixWorld(true);
  return { group, mesh, camera };
}

describe("CAD vertex catalogue", () => {
  it("reads points and keeps only vertex references", () => {
    const { mesh } = scene();
    const vertices = cadVertices(mesh)!;
    expect(vertices).toHaveLength(2);
    expect(vertices[0].topologyRef?.role).toBe("box-vertex:xmax:ymin:zmax");
    expect(vertices[1].topologyRef).toBeNull();
    const broken = new THREE.Mesh(new THREE.BufferGeometry());
    broken.geometry.userData.formaVertices = [
      { topologyRef: { ...reference, kind: "edge" }, point: [0, 0, 0] },
      { point: [0, Number.NaN, 0] },
    ];
    expect(cadVertices(broken)).toBeNull();
  });
  it("picks the nearest projected vertex within the radius in world millimetres", () => {
    const { group, camera } = scene();
    const size = { width: 100, height: 100 };
    // World x = 10 mm projects to screen x = 60 px.
    const hit = pickCadVertex(group, camera, { x: 58, y: 50 }, size)!;
    expect(hit.vertex.ordinal).toBe(1);
    expect(hit.world.x).toBeCloseTo(10);
    expect(pickCadVertex(group, camera, { x: 80, y: 50 }, size)).toBeNull();
    expect(
      pickCadVertex(
        group,
        camera,
        { x: 58, y: 50 },
        size,
        14,
        new Set(["body"]),
      ),
    ).toBeNull();
  });
});
