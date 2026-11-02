import { Matrix4, Vector3, type Object3D } from "three";
import type { Vec3 } from "../frames";
/**
 * The native preview writes CAD (x, y, z) mm as glTF (x, z, -y) metres and the
 * loader scales its root by 1000, so the scene is Y-up while CAD IR and
 * TransformBody stay Z-up.
 */
// prettier-ignore
const cadToRender = new Matrix4().set(
  1, 0, 0, 0,
  0, 0, 1, 0,
  0, -1, 0, 0,
  0, 0, 0, 1,
);

/** CAD mm → world for a loaded native preview root, including any display offset of the root. */
export function cadWorldFrame(root?: Object3D | null): Matrix4 {
  if (!root) return cadToRender.clone();
  root.updateWorldMatrix(true, false);
  return root.matrixWorld
    .clone()
    .multiply(new Matrix4().makeScale(0.001, 0.001, 0.001))
    .multiply(cadToRender);
}

export const cadPointToRender = (point: Vec3, frame = cadToRender): Vector3 =>
  new Vector3(...point).applyMatrix4(frame);
export const renderPointToCad = (point: Vector3, frame = cadToRender): Vec3 => {
  const value = point.clone().applyMatrix4(frame.clone().invert());
  return [value.x, value.y, value.z];
};
export const renderDirectionToCad = (
  direction: Vector3,
  frame = cadToRender,
): Vec3 => {
  const value = direction.clone().transformDirection(frame.clone().invert());
  return [value.x, value.y, value.z];
};
/** Conjugates a CAD-space delta into world space: F · Δ · F⁻¹. */
export function cadDeltaToRender(delta: Matrix4, frame = cadToRender): Matrix4 {
  return frame.clone().multiply(delta).multiply(frame.clone().invert());
}
