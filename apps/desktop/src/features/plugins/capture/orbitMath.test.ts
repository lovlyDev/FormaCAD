import * as THREE from "three";
import { describe, expect, it } from "vitest";
import { bitrate, cameraPosition, frameCount, frameQuaternion, standQuaternion, turntableAngle, type OrbitSpec } from "./orbitMath";

const spec: OrbitSpec = {
  width: 1920,
  height: 1080,
  fps: 30,
  durationMs: 4000,
  turns: 1,
  clockwise: false,
  up: "z",
  tiltDeg: 0,
  elevationDeg: 20,
  startDeg: 0,
  zoom: 1,
  background: "theme",
  format: "mp4",
  bitrateKbps: 0,
};

describe("orbit math", () => {
  it("covers whole turns without repeating the first frame", () => {
    expect(frameCount(spec)).toBe(120);
    expect(turntableAngle(spec, 0)).toBe(0);
    expect(turntableAngle(spec, 60)).toBeCloseTo(Math.PI);
    expect(turntableAngle(spec, 120)).toBeCloseTo(2 * Math.PI);
    expect(turntableAngle({ ...spec, clockwise: true, turns: 2 }, 30)).toBeCloseTo(-Math.PI);
  });

  it("stands the chosen CAD axis upright", () => {
    const up = new THREE.Vector3(0, 1, 0);
    // CAD Z is already up in the Y-up viewer scene.
    expect(new THREE.Vector3(0, 1, 0).applyQuaternion(standQuaternion(spec)).distanceTo(up)).toBeLessThan(1e-6);
    // CAD X (viewer +X) becomes up.
    expect(new THREE.Vector3(1, 0, 0).applyQuaternion(standQuaternion({ ...spec, up: "x" })).distanceTo(up)).toBeLessThan(1e-6);
    // CAD −Y (viewer +Z) becomes up.
    expect(new THREE.Vector3(0, 0, 1).applyQuaternion(standQuaternion({ ...spec, up: "-y" })).distanceTo(up)).toBeLessThan(1e-6);
  });

  it("spins about the vertical after tilting", () => {
    const tilted = { ...spec, tiltDeg: 90 };
    const top = new THREE.Vector3(0, 1, 0).applyQuaternion(frameQuaternion(tilted, 0));
    expect(top.distanceTo(new THREE.Vector3(0, 0, 1))).toBeLessThan(1e-6);
    const quarter = new THREE.Vector3(0, 1, 0).applyQuaternion(frameQuaternion(tilted, 30));
    expect(quarter.distanceTo(new THREE.Vector3(1, 0, 0))).toBeLessThan(1e-6);
  });

  it("frames the bounding sphere and honours elevation and zoom", () => {
    const camera = cameraPosition(spec, 10);
    expect(Math.atan2(camera.y, camera.z)).toBeCloseTo((20 * Math.PI) / 180);
    expect(camera.length()).toBeGreaterThan(10 / Math.sin((16 * Math.PI) / 180));
    expect(cameraPosition({ ...spec, zoom: 2 }, 10).length()).toBeCloseTo(camera.length() / 2);
    // A portrait frame is limited by its narrower horizontal field of view.
    expect(cameraPosition({ ...spec, width: 1080, height: 1920 }, 10).length()).toBeGreaterThan(camera.length());
  });

  it("chooses a bitrate from the pixel rate unless one is given", () => {
    expect(bitrate(spec)).toBe(Math.round(1920 * 1080 * 30 * 0.12));
    expect(bitrate({ ...spec, width: 160, height: 120, fps: 1 })).toBe(1_000_000);
    expect(bitrate({ ...spec, bitrateKbps: 5000 })).toBe(5_000_000);
  });
});
