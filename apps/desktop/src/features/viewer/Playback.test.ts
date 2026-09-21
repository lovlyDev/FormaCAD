import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { hasMotion } from "./Playback";

describe("conditional motion controls", () => {
  it("detects only models with a nonzero motion script", () => {
    const model = new THREE.Group();
    expect(hasMotion(model)).toBe(false);
    const staticPart = new THREE.Object3D();
    staticPart.userData.formaMotion = { axis: [0, 1, 0], speed: 0 };
    model.add(staticPart);
    expect(hasMotion(model)).toBe(false);
    const movingPart = new THREE.Object3D();
    movingPart.userData.formaMotion = { axis: [0, 1, 0], speed: 24 };
    model.add(movingPart);
    expect(hasMotion(model)).toBe(true);
  });
});
