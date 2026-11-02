import { expect, it } from "vitest";
import { Vector3 } from "three";
import { standardViewPose, type StandardView } from "./standardViews";
it("provides exact axial and opposite views without a singular top/bottom camera", () => {
  const target = new Vector3(10, 20, 30);
  for (const [positive, negative] of [
    ["front", "back"],
    ["right", "left"],
    ["top", "bottom"],
  ] as [StandardView, StandardView][]) {
    const a = standardViewPose(positive, target, 100)!,
      b = standardViewPose(negative, target, 100)!;
    expect(a.position.distanceTo(target)).toBeCloseTo(100);
    expect(
      a.position
        .clone()
        .sub(target)
        .add(b.position.clone().sub(target))
        .length(),
    ).toBeCloseTo(0);
    expect(a.position.clone().sub(target).normalize().dot(a.up)).toBeCloseTo(0);
  }
  expect(target.toArray()).toEqual([10, 20, 30]);
  expect(standardViewPose("front", target, 0)).toBeNull();
});
