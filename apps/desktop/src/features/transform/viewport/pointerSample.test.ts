import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { pointerSample, projectToScreen, unwrapAngle } from "./pointerSample";

const size = { width: 800, height: 600 };
function topCamera() {
  // Looks down render -Y, i.e. along CAD -Z: screen right is CAD +X, screen up is CAD +Y.
  const camera = new THREE.OrthographicCamera(-400, 400, 300, -300, 0.1, 1000);
  camera.position.set(0, 100, 0);
  camera.up.set(0, 0, -1);
  camera.lookAt(0, 0, 0);
  camera.updateMatrixWorld();
  return camera;
}

describe("modal pointer sampling", () => {
  it("drags on the view plane in CAD millimetres", () => {
    const camera = topCamera();
    const centre = projectToScreen(camera, size, [0, 0, 0]);
    expect(centre.x).toBeCloseTo(400);
    const sample = pointerSample({
      camera,
      size,
      start: centre,
      current: { x: centre.x + 10, y: centre.y - 20 },
      pivotMm: [0, 0, 0],
      rotationAxisMm: [0, 0, 1],
    })!;
    expect(sample.translationMm[0]).toBeCloseTo(10);
    expect(sample.translationMm[1]).toBeCloseTo(20);
    expect(sample.translationMm[2]).toBeCloseTo(0);
  });
  it("turns counter-clockwise positive around an axis facing the viewer", () => {
    const camera = topCamera();
    const sample = pointerSample({
      camera,
      size,
      start: { x: 500, y: 300 },
      current: { x: 400, y: 200 },
      pivotMm: [0, 0, 0],
      rotationAxisMm: [0, 0, 1],
    })!;
    expect(sample.angleDeg).toBeCloseTo(90);
    expect(sample.scaleDelta).toBeCloseTo(0);
  });
  it("scales by the radius ratio and unwraps full turns", () => {
    const camera = topCamera();
    const sample = pointerSample({
      camera,
      size,
      start: { x: 450, y: 300 },
      current: { x: 500, y: 300 },
      pivotMm: [0, 0, 0],
      rotationAxisMm: [0, 0, 1],
    })!;
    expect(sample.scaleDelta).toBeCloseTo(1);
    expect(unwrapAngle(170, -170)).toBeCloseTo(190);
    expect(unwrapAngle(-350, 20)).toBeCloseTo(-340);
  });
});
