import * as THREE from "three";
import { readTopologyReference, type EdgeTopologyReference } from "./topology/topologyReference";

export interface EdgeSelection {
  bodyId: string;
  edgeOrdinal: number;
  revisionId: string;
  sceneToken?: string;
  semanticKey?: string;
  topologyRef?: EdgeTopologyReference;
}

export interface CadEdge {
  lengthMm: number;
  radiusMm: number | null;
  points: [number, number, number][];
  semanticKey: string | null;
  topologyRef?: EdgeTopologyReference;
}

const parsedEdges = new WeakMap<
  THREE.BufferGeometry,
  { source: unknown; edges: CadEdge[] | null }
>();

export function cadEdges(mesh: THREE.Mesh): CadEdge[] | null {
  const raw: unknown = mesh.geometry.userData.formaEdges;
  const cached = parsedEdges.get(mesh.geometry);
  if (cached && cached.source === raw) return cached.edges;
  const parsed = parseCadEdges(raw);
  parsedEdges.set(mesh.geometry, { source: raw, edges: parsed });
  return parsed;
}

function parseCadEdges(raw: unknown): CadEdge[] | null {
  if (!Array.isArray(raw) || raw.length === 0 || raw.length > 100_000)
    return null;
  const result: CadEdge[] = [];
  for (const entry of raw) {
    if (!entry || typeof entry !== "object") return null;
    const candidate = entry as Record<string, unknown>;
    if (
      typeof candidate.lengthMm !== "number" ||
      !Number.isFinite(candidate.lengthMm) ||
      candidate.lengthMm < 0 ||
      !Array.isArray(candidate.points) ||
      candidate.points.length > 49
    )
      return null;
    const points: [number, number, number][] = [];
    for (const point of candidate.points) {
      if (
        !Array.isArray(point) ||
        point.length !== 3 ||
        !point.every(
          (value) => typeof value === "number" && Number.isFinite(value),
        )
      )
        return null;
      points.push([point[0], point[1], point[2]]);
    }
    if (points.length === 1) return null;
    const radius = candidate.radiusMm;
    const semanticKey = candidate.semanticKey;
    const reference = candidate.topologyRef == null ? null : readTopologyReference(candidate.topologyRef);
    if (candidate.topologyRef != null && (!reference || reference.kind !== "edge")) return null;
    if (semanticKey !== undefined && semanticKey !== null &&
      (typeof semanticKey !== "string" || !/^box-edge:[xyz]:[xyz](min|max):[xyz](min|max)$/.test(semanticKey))) return null;
    if (
      radius !== undefined &&
      radius !== null &&
      (typeof radius !== "number" || !Number.isFinite(radius) || radius <= 0)
    )
      return null;
    result.push({
      lengthMm: candidate.lengthMm,
      radiusMm: typeof radius === "number" ? radius : null,
      points,
      semanticKey: typeof semanticKey === "string" ? semanticKey : null,
      ...(reference?.kind === "edge" ? { topologyRef: reference } : {}),
    });
  }
  return result;
}

export function edgeLengthMm(mesh: THREE.Mesh, ordinal: number): number | null {
  const edges = cadEdges(mesh);
  if (
    !edges ||
    !Number.isSafeInteger(ordinal) ||
    ordinal < 1 ||
    ordinal > edges.length
  )
    return null;
  return edges[ordinal - 1].lengthMm;
}

export function edgeRadiusMm(mesh: THREE.Mesh, ordinal: number): number | null {
  const edges = cadEdges(mesh);
  if (
    !edges ||
    !Number.isSafeInteger(ordinal) ||
    ordinal < 1 ||
    ordinal > edges.length
  )
    return null;
  return edges[ordinal - 1].radiusMm;
}

export function edgeOverlay(
  mesh: THREE.Mesh,
  edge: CadEdge,
  ordinal: number,
  selected: boolean,
): THREE.LineSegments | null {
  if (edge.points.length < 2) return null;
  const coordinates: number[] = [];
  for (let index = 1; index < edge.points.length; index++)
    coordinates.push(...edge.points[index - 1], ...edge.points[index]);
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute(
    "position",
    new THREE.Float32BufferAttribute(coordinates, 3),
  );
  const overlay = new THREE.LineSegments(
    geometry,
    new THREE.LineBasicMaterial({
      color: selected ? "#f2a43a" : "#708d9f",
      depthTest: !selected,
      transparent: true,
      opacity: selected ? 1 : 0.8,
    }),
  );
  overlay.userData.formaSelectionOverlay = true;
  overlay.userData.formaEdgeOrdinal = ordinal;
  if (edge.semanticKey) overlay.userData.formaEdgeSemanticKey = edge.semanticKey;
  overlay.userData.formaBodyId = mesh.name;
  overlay.renderOrder = selected ? 10 : 1;
  return overlay;
}
