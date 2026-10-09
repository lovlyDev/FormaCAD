import { describe, expect, it } from "vitest";
import * as THREE from "three";
import {
  faceAreaMm2,
  faceForTriangle,
  faceOverlay,
  faceTriangleCounts,
} from "./faceSelection";

describe("native CAD face mapping", () => {
  it("maps triangles to the correct face and isolates its overlay", () => {
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute(
      "position",
      new THREE.Float32BufferAttribute(
        [0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 1, 0, 2, 1, 0, 1, 2, 0],
        3,
      ),
    );
    geometry.setIndex([0, 1, 2, 3, 4, 5]);
    geometry.userData.formaFaceTriangleCounts = [0, 1, 1];
    geometry.userData.formaFaceAreasMm2 = [0, 12.5, 7.75];
    const mesh = new THREE.Mesh(geometry);
    const counts = faceTriangleCounts(mesh);
    expect(counts).toEqual([0, 1, 1]);
    expect(faceForTriangle(counts!, 0)).toBe(2);
    expect(faceForTriangle(counts!, 1)).toBe(3);
    expect(faceForTriangle(counts!, 2)).toBeNull();
    expect(faceAreaMm2(mesh, 2)).toBe(12.5);
    expect(faceAreaMm2(mesh, 3)).toBe(7.75);
    expect(faceAreaMm2(mesh, 1)).toBeNull();
    const overlay = faceOverlay(mesh, 3);
    expect(Array.from(overlay!.geometry.index!.array)).toEqual([3, 4, 5]);
    overlay!.geometry.dispose();
    (overlay!.material as THREE.Material).dispose();
    geometry.dispose();
  });

  it("rejects preview metadata that does not match mesh triangles", () => {
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute(
      "position",
      new THREE.Float32BufferAttribute([0, 0, 0, 1, 0, 0, 0, 1, 0], 3),
    );
    geometry.setIndex([0, 1, 2]);
    geometry.userData.formaFaceTriangleCounts = [2];
    geometry.userData.formaFaceAreasMm2 = [999];
    const mesh = new THREE.Mesh(geometry);
    expect(faceTriangleCounts(mesh)).toBeNull();
    expect(faceAreaMm2(mesh, 1)).toBeNull();
    geometry.dispose();
  });
});
