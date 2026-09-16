import { describe, it, expect } from "vitest";
import * as THREE from "three";
import { ThreeMFLoader } from "three/examples/jsm/loaders/3MFLoader.js";
import { encodeThreeMf } from "./threeMf";
describe("3MF export", () => {
  it("round trips dimensions, transformed bodies and visibility", () => {
    const source = new THREE.Group();
    const body = new THREE.Mesh(new THREE.BoxGeometry(10, 20, 30));
    body.position.set(15, 10, -2);
    source.add(body);
    const hidden = new THREE.Mesh(new THREE.BoxGeometry(500, 500, 500));
    hidden.visible = false;
    source.add(hidden);
    const bytes = encodeThreeMf(source);
    const loaded = new ThreeMFLoader().parse(new Uint8Array(bytes).buffer);
    loaded.rotateX(-Math.PI / 2);
    loaded.updateMatrixWorld(true);
    const bounds = new THREE.Box3().setFromObject(loaded);
    const size = bounds.getSize(new THREE.Vector3());
    expect(size.x).toBeCloseTo(10);
    expect(size.y).toBeCloseTo(20);
    expect(size.z).toBeCloseTo(30);
    expect(bounds.getCenter(new THREE.Vector3()).x).toBeCloseTo(15);
    let triangles = 0;
    loaded.traverse((o) => {
      if (o instanceof THREE.Mesh)
        triangles +=
          (o.geometry.index?.count ??
            o.geometry.getAttribute("position").count) / 3;
    });
    expect(triangles).toBe(12);
  });
});
