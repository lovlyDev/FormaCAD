import { Plane, type Object3D, Vector3 } from "three";
const planes = new WeakMap<Object3D, Plane>();
export function setSectionPlane(object: Object3D, plane: Plane | null) { if (plane) planes.set(object, plane); else planes.delete(object); }
export function getSectionPlane(object: Object3D): Plane | undefined { return planes.get(object); }
export function clipSegment(a: Vector3, b: Vector3, plane: Plane | undefined): boolean {
  if (!plane) return true;
  const first = plane.distanceToPoint(a), second = plane.distanceToPoint(b);
  if (first < 0 && second < 0) return false;
  if (first < 0 || second < 0) {
    const cut = a.clone().lerp(b, first / (first - second));
    if (first < 0) a.copy(cut); else b.copy(cut);
  }
  return true;
}
