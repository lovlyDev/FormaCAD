import * as THREE from "three";
import { readTopologyReference, type FaceTopologyReference } from "./topology/topologyReference";

export interface FaceSelection {
  bodyId: string;
  faceOrdinal: number;
  revisionId: string;
  sceneToken?: string;
  topologyRef?: FaceTopologyReference;
}

export function faceReference(mesh: THREE.Mesh, ordinal: number): FaceTopologyReference | null {
  const counts = faceTriangleCounts(mesh);
  const references: unknown = mesh.geometry.userData.formaFaceReferences;
  if (!counts || !Array.isArray(references) || references.length !== counts.length || !Number.isSafeInteger(ordinal) || ordinal < 1 || ordinal > references.length) return null;
  const reference = readTopologyReference(references[ordinal - 1]);
  return reference?.kind === "face" ? reference : null;
}

export function faceTriangleCounts(mesh: THREE.Mesh): number[] | null {
  const value: unknown = mesh.geometry.userData.formaFaceTriangleCounts;
  if (!Array.isArray(value) || !value.length) return null;
  const counts: number[] = [];
  let total = 0;
  for (const count of value) {
    if (!Number.isSafeInteger(count) || count < 0) return null;
    counts.push(count);
    total += count;
  }
  const indexCount =
    mesh.geometry.index?.count ?? mesh.geometry.getAttribute("position")?.count;
  return indexCount === total * 3 ? counts : null;
}

export function faceForTriangle(
  counts: number[],
  triangle: number,
): number | null {
  if (!Number.isSafeInteger(triangle) || triangle < 0) return null;
  let end = 0;
  for (let face = 0; face < counts.length; face++) {
    end += counts[face];
    if (triangle < end) return face + 1;
  }
  return null;
}

export function faceAreaMm2(
  mesh: THREE.Mesh,
  faceOrdinal: number,
): number | null {
  const counts = faceTriangleCounts(mesh);
  const values: unknown = mesh.geometry.userData.formaFaceAreasMm2;
  if (!counts || !Array.isArray(values) || values.length !== counts.length)
    return null;
  if (
    !Number.isSafeInteger(faceOrdinal) ||
    faceOrdinal < 1 ||
    faceOrdinal > counts.length
  )
    return null;
  const area: unknown = values[faceOrdinal - 1];
  return typeof area === "number" && Number.isFinite(area) && area > 0
    ? area
    : null;
}

export function faceTriangleRange(
  counts: number[],
  faceOrdinal: number,
): [number, number] | null {
  if (
    !Number.isSafeInteger(faceOrdinal) ||
    faceOrdinal < 1 ||
    faceOrdinal > counts.length
  )
    return null;
  let start = 0;
  for (let index = 0; index < faceOrdinal - 1; index++) start += counts[index];
  return [start, start + counts[faceOrdinal - 1]];
}

export function faceOverlay(
  mesh: THREE.Mesh,
  faceOrdinal: number,
): THREE.Mesh | null {
  const counts = faceTriangleCounts(mesh);
  const range = counts && faceTriangleRange(counts, faceOrdinal);
  if (!range || range[0] === range[1]) return null;
  const source = mesh.geometry;
  const geometry = new THREE.BufferGeometry();
  for (const name of Object.keys(source.attributes))
    geometry.setAttribute(name, source.getAttribute(name));
  const indices = source.index;
  const selected = new Uint32Array((range[1] - range[0]) * 3);
  for (let index = 0; index < selected.length; index++) {
    selected[index] = indices
      ? indices.getX(range[0] * 3 + index)
      : range[0] * 3 + index;
  }
  geometry.setIndex(new THREE.BufferAttribute(selected, 1));
  const overlay = new THREE.Mesh(
    geometry,
    new THREE.MeshBasicMaterial({
      color: "#f2a43a",
      side: THREE.DoubleSide,
      transparent: true,
      opacity: 0.72,
      depthWrite: false,
      polygonOffset: true,
      polygonOffsetFactor: -2,
      polygonOffsetUnits: -2,
    }),
  );
  overlay.renderOrder = 5;
  overlay.userData.formaSelectionOverlay = true;
  overlay.raycast = () => {};
  return overlay;
}
