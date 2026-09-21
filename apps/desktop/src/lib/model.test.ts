import * as THREE from "three";
import { describe, it, expect } from "vitest";
import {
  buildModel,
  inspectModel,
  parameterSchema,
  disposeModel,
} from "./model";
import { defaults } from "../types";
import { exportMesh, loadModel } from "./files";
describe("CAD model integrity", () => {
  it.each(["box", "bracket", "plate", "enclosure", "cylinder"] as const)(
    "builds finite nonzero geometry for %s",
    (kind) => {
      const m = buildModel({ ...defaults, kind });
      const info = inspectModel(m);
      expect(info.triangles).toBeGreaterThan(0);
      expect(info.volume).toBeGreaterThan(0);
      expect(info.size.every((x) => Number.isFinite(x) && x > 0)).toBe(true);
      disposeModel(m);
    },
  );
  it("represents bracket bounds and holes", () => {
    const m = buildModel(defaults);
    const info = inspectModel(m);
    expect(info.size[0]).toBeCloseTo(120);
    expect(info.size[1]).toBeCloseTo(60);
    expect(info.size[2]).toBeCloseTo(65);
    const solid = buildModel({ ...defaults, holes: 0 });
    expect(inspectModel(solid).volume).toBeGreaterThan(info.volume);
    disposeModel(m);
    disposeModel(solid);
  });
  it("rejects invalid or unsafe parameters", () => {
    for (const patch of [
      { width: 0 },
      { width: Infinity },
      { width: NaN },
      { holes: 99 },
      { holes: 1.5 },
      { thickness: 50 },
      { holeDiameter: 100 },
    ])
      expect(parameterSchema.safeParse({ ...defaults, ...patch }).success).toBe(
        false,
      );
  });
  it("exports actual STL facets and OBJ vertices", async () => {
    const m = buildModel(defaults);
    const stl = await exportMesh(m, "stl");
    const obj = await exportMesh(m, "obj");
    expect(stl.size).toBeGreaterThan(1000);
    expect(obj.size).toBeGreaterThan(1000);
    disposeModel(m);
  });
  it("rejects missing model bytes", async () => {
    await expect(
      loadModel({ name: "bad.stl", kind: "model", size: 0 }),
    ).rejects.toThrow("missing");
  });
});

it("cuts a centered 20 mm through-hole in a 50 mm cube", () => {
  const solid = buildModel({
    ...defaults,
    kind: "box",
    width: 50,
    depth: 50,
    height: 50,
    holes: 0,
    holeDiameter: 0,
  });
  const cut = buildModel({
    ...defaults,
    kind: "box",
    width: 50,
    depth: 50,
    height: 50,
    holes: 1,
    holeDiameter: 20,
  });
  // The viewer approximates the circular wall with 96 segments (error < 0.1%).
  const removed=inspectModel(solid).volume-inspectModel(cut).volume;
  expect(Math.abs(removed/(Math.PI*100*50)-1)).toBeLessThan(.001);
  const ray = new THREE.Raycaster(
    new THREE.Vector3(0, 100, 0),
    new THREE.Vector3(0, -1, 0),
  );
  expect(ray.intersectObject(cut, true)).toHaveLength(0);
  ray.set(new THREE.Vector3(15, 100, 0), new THREE.Vector3(0, -1, 0));
  expect(ray.intersectObject(cut, true).length).toBeGreaterThan(0);
  disposeModel(solid);
  disposeModel(cut);
});
