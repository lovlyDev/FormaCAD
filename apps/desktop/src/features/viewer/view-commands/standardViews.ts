import { Vector3 } from "three";
export type StandardView =
  "iso" | "front" | "back" | "right" | "left" | "top" | "bottom";
// Render space is Y-up; the existing CAD-to-render conversion stays at the scene boundary.
const directions: Record<StandardView, [number, number, number]> = {
  iso: [1, 0.8, 1],
  front: [0, 0, 1],
  back: [0, 0, -1],
  right: [1, 0, 0],
  left: [-1, 0, 0],
  top: [0, 1, 0],
  bottom: [0, -1, 0],
};
export function standardViewPose(
  view: StandardView,
  target: Vector3,
  distance: number,
): { position: Vector3; target: Vector3; up: Vector3 } | null {
  if (
    ![target.x, target.y, target.z, distance].every(Number.isFinite) ||
    distance <= 0 ||
    distance > 1e9
  )
    return null;
  const direction = new Vector3(...directions[view]).normalize();
  const up =
    view === "top"
      ? new Vector3(0, 0, -1)
      : view === "bottom"
        ? new Vector3(0, 0, 1)
        : new Vector3(0, 1, 0);
  return {
    position: target.clone().addScaledVector(direction, distance),
    target: target.clone(),
    up,
  };
}
