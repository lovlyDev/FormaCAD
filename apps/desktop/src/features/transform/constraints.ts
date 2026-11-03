import {
  type Axis,
  type TransformFrame,
  type Vec3,
  tuple,
  vector,
} from "./frames";
export interface AxisConstraint {
  axis: Axis;
  exclude: boolean;
  space: "orientation" | "local";
}
/** First press uses the chosen orientation, repeat uses the body's local frame, third releases. */
export function cycleConstraint(
  current: AxisConstraint | null,
  axis: Axis,
  exclude = false,
): AxisConstraint | null {
  if (!current || current.axis !== axis || current.exclude !== exclude)
    return { axis, exclude, space: "orientation" };
  return current.space === "orientation"
    ? { ...current, space: "local" }
    : null;
}
export function constrainVector(
  delta: Vec3,
  constraint: AxisConstraint | null,
  orientation: TransformFrame,
  local: TransformFrame,
): Vec3 {
  if (!constraint) return [...delta];
  const axis = vector(
    (constraint.space === "local" ? local : orientation)[constraint.axis],
  );
  const projection = axis.multiplyScalar(vector(delta).dot(axis));
  return tuple(constraint.exclude ? vector(delta).sub(projection) : projection);
}
export function constrainedDirection(
  constraint: AxisConstraint | null,
  orientation: TransformFrame,
  local: TransformFrame,
  motion: Vec3,
): Vec3 {
  if (constraint && !constraint.exclude)
    return [
      ...(constraint.space === "local" ? local : orientation)[constraint.axis],
    ];
  const direction = vector(
    constrainVector(motion, constraint, orientation, local),
  );
  if (direction.lengthSq() < 1e-18) {
    const frame = constraint?.space === "local" ? local : orientation;
    direction.fromArray(
      constraint?.exclude && constraint.axis === "x" ? frame.y : frame.x,
    );
  }
  return tuple(direction.normalize());
}
