import { expect, it } from "vitest";
import { Vector3 } from "three";
import { cadToWorld, worldToCad, clipPlane } from "./sectionPlane";
import { clipSegment } from "./sectionClipping";
it("maps CAD mm into the already millimetre-scaled workspace without an extra factor of 1000", () => {
  expect(cadToWorld([40, 20, 10])).toEqual([40, 10, -20]);
  expect(worldToCad(cadToWorld([40, 20, 10]))).toEqual([40, 20, 10]);
  const plane = clipPlane({ originMm: [0, 0, 5], normal: [0, 0, 2], deflectionMm: 0.01 });
  expect(plane.distanceToPoint(new Vector3(...cadToWorld([10, 20, 5])))).toBe(0);
  expect(plane.distanceToPoint(new Vector3(...cadToWorld([10, 20, 4])))).toBeLessThan(0);
});
it("removes hidden edges and trims crossing edges to the same visible clip half-space", () => {
  const plane = clipPlane({ originMm: [0, 0, 5], normal: [0, 0, 1], deflectionMm: 0.01 });
  expect(clipSegment(new Vector3(0, 1, 0), new Vector3(0, 2, 0), plane)).toBe(false);
  const a = new Vector3(0, 1, 0), b = new Vector3(0, 10, 0); expect(clipSegment(a, b, plane)).toBe(true);
  expect(a.y).toBeCloseTo(5); expect(b.y).toBe(10);
});
