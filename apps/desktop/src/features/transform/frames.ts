import { Matrix4, Vector3 } from "three";
export type Vec3 = [number, number, number];
export type Axis = "x" | "y" | "z";
export interface TransformFrame {
  x: Vec3;
  y: Vec3;
  z: Vec3;
}
export const worldFrame: TransformFrame = {
  x: [1, 0, 0],
  y: [0, 1, 0],
  z: [0, 0, 1],
};
export const vector = (value: Vec3) => new Vector3(...value);
export const tuple = (value: Vector3): Vec3 => [value.x, value.y, value.z];
export function finiteVector(value: readonly number[]): boolean {
  return value.length === 3 && value.every(Number.isFinite);
}
/** Build a right-handed frame; axes contain directions, never scale/shear. */
export function normalFrame(
  normal: Vec3,
  preferredX: Vec3 = [1, 0, 0],
): TransformFrame | null {
  if (!finiteVector(normal) || !finiteVector(preferredX)) return null;
  const z = vector(normal);
  if (!Number.isFinite(z.lengthSq()) || z.lengthSq() < 1e-18) return null;
  z.normalize();
  const x = vector(preferredX).addScaledVector(z, -vector(preferredX).dot(z));
  if (!Number.isFinite(x.lengthSq())) return null;
  if (x.lengthSq() < 1e-12) {
    x.set(Math.abs(z.x) < 0.9 ? 1 : 0, Math.abs(z.x) < 0.9 ? 0 : 1, 0);
    x.addScaledVector(z, -x.dot(z));
  }
  x.normalize();
  return {
    x: tuple(x),
    y: tuple(new Vector3().crossVectors(z, x).normalize()),
    z: tuple(z),
  };
}
export function frameFromMatrix(
  elements: readonly number[],
): TransformFrame | null {
  if (elements.length !== 16 || !elements.every(Number.isFinite)) return null;
  const matrix = new Matrix4().fromArray(elements);
  if (
    !Number.isFinite(matrix.determinant()) ||
    Math.abs(matrix.determinant()) < 1e-18
  )
    return null;
  return normalFrame(
    tuple(new Vector3().setFromMatrixColumn(matrix, 2)),
    tuple(new Vector3().setFromMatrixColumn(matrix, 0)),
  );
}
export function validWorldMatrix(elements: readonly number[]): boolean {
  return (
    elements.length === 16 &&
    elements.every(Number.isFinite) &&
    [3, 7, 11].every((index) => Math.abs(elements[index]) < 1e-12) &&
    Math.abs(elements[15] - 1) < 1e-12 &&
    Number.isFinite(new Matrix4().fromArray(elements).determinant()) &&
    Math.abs(new Matrix4().fromArray(elements).determinant()) > 1e-18
  );
}
export function validFrame(frame: TransformFrame): boolean {
  const axes = [frame.x, frame.y, frame.z];
  if (!axes.every(finiteVector)) return false;
  const [x, y, z] = axes.map(vector);
  return (
    [x, y, z].every((axis) => Math.abs(axis.lengthSq() - 1) < 1e-8) &&
    Math.abs(x.dot(y)) < 1e-8 &&
    Math.abs(y.dot(z)) < 1e-8 &&
    Math.abs(z.dot(x)) < 1e-8 &&
    new Vector3().crossVectors(x, y).dot(z) > 1 - 1e-8
  );
}
