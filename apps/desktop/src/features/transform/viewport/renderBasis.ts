import { Matrix4, Vector3 } from "three";
import type { Vec3 } from "../frames";
/**
 * The native preview writes CAD (x, y, z) mm as render (x, z, -y) mm, so the
 * scene is Y-up while CAD IR and TransformBody stay Z-up.
 */
// prettier-ignore
const cadToRender = new Matrix4().set(
  1, 0, 0, 0,
  0, 0, 1, 0,
  0, -1, 0, 0,
  0, 0, 0, 1,
);
const renderToCad = cadToRender.clone().invert();
export const cadPointToRender = (point: Vec3): Vector3 =>
  new Vector3(...point).applyMatrix4(cadToRender);
export const renderPointToCad = (point: Vector3): Vec3 => {
  const value = point.clone().applyMatrix4(renderToCad);
  return [value.x, value.y, value.z];
};
export const renderDirectionToCad = (direction: Vector3): Vec3 => {
  const value = direction.clone().transformDirection(renderToCad);
  return [value.x, value.y, value.z];
};
/** Conjugates a CAD-space delta into the render basis: B · Δ · B⁻¹. */
export function cadDeltaToRender(delta: Matrix4): Matrix4 {
  return cadToRender.clone().multiply(delta).multiply(renderToCad);
}
