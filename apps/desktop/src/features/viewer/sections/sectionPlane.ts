import { Plane, Vector3 } from "three";
export type VectorMm = [number, number, number];
export type SectionPlane = { originMm: VectorMm; normal: VectorMm; deflectionMm: number };
// GLB loaders scale metres back to millimetres; the workspace uses CAD mm.
export const cadToWorld = ([x, y, z]: VectorMm): VectorMm => [x, z, -y];
export const worldToCad = ([x, y, z]: VectorMm): VectorMm => [x, -z, y];
export function clipPlane(plane: SectionPlane): Plane {
  const normal = new Vector3(...cadToWorld(plane.normal)).normalize();
  return new Plane(normal, -normal.dot(new Vector3(...cadToWorld(plane.originMm))));
}
export const sectionNormals: Record<"xy" | "xz" | "yz", VectorMm> = { xy: [0, 0, 1], xz: [0, 1, 0], yz: [1, 0, 0] };
