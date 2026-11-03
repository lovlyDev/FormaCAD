import type { Vec3 } from "../transform/frames";
import { finiteVector } from "../transform/frames";
/** Grid and increment are mathematical constraints; these do not claim a BREP hit. */
export function snapIncrement(value: number, step: number): number | null {
  if (
    !Number.isFinite(value) ||
    !Number.isFinite(step) ||
    step <= 0 ||
    step > 10000
  )
    return null;
  const rounded = Math.round(value / step) * step;
  return Number.isFinite(rounded) ? Number(rounded.toPrecision(15)) : null;
}
export function snapGrid(
  pointMm: Vec3,
  originMm: Vec3,
  stepMm: number,
): Vec3 | null {
  if (!finiteVector(pointMm) || !finiteVector(originMm)) return null;
  const snapped = pointMm.map((value, axis) =>
    snapIncrement(value - originMm[axis], stepMm),
  );
  if (snapped.some((value) => value === null)) return null;
  return snapped.map((value, axis) => value! + originMm[axis]) as Vec3;
}
