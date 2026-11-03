import * as THREE from "three";
import {
  readTopologyReference,
  type VertexTopologyReference,
} from "./topology/topologyReference";

/** One BREP vertex from the native preview; `point` is in the mesh's local glTF metres. */
export interface CadVertex {
  ordinal: number;
  point: THREE.Vector3;
  topologyRef: VertexTopologyReference | null;
}

const cache = new WeakMap<THREE.BufferGeometry, CadVertex[] | null>();

export function cadVertices(mesh: THREE.Mesh): CadVertex[] | null {
  if (cache.has(mesh.geometry)) return cache.get(mesh.geometry)!;
  const raw: unknown = mesh.geometry.userData.formaVertices;
  let result: CadVertex[] | null = null;
  if (Array.isArray(raw) && raw.length <= 20000) {
    result = [];
    for (const [index, item] of raw.entries()) {
      const point = item?.point;
      if (
        !Array.isArray(point) ||
        point.length !== 3 ||
        !point.every(
          (value: unknown) =>
            typeof value === "number" && Number.isFinite(value),
        )
      ) {
        result = null;
        break;
      }
      const reference =
        item.topologyRef == null
          ? null
          : readTopologyReference(item.topologyRef);
      // A malformed or non-vertex reference is dropped, never trusted.
      result.push({
        ordinal: index + 1,
        point: new THREE.Vector3(point[0], point[1], point[2]),
        topologyRef: reference?.kind === "vertex" ? reference : null,
      });
    }
  }
  cache.set(mesh.geometry, result);
  return result;
}

export interface VertexHit {
  bodyId: string;
  vertex: CadVertex;
  /** World-space render position. */
  world: THREE.Vector3;
  distancePx: number;
}

/** Nearest visible preview vertex to a screen point, within `radiusPx`. */
export function pickCadVertex(
  object: THREE.Object3D,
  camera: THREE.Camera,
  screen: { x: number; y: number },
  size: { width: number; height: number },
  radiusPx = 14,
  exclude: ReadonlySet<string> = new Set(),
): VertexHit | null {
  let best: VertexHit | null = null;
  const projected = new THREE.Vector3();
  object.traverse((node) => {
    if (
      !(node instanceof THREE.Mesh) ||
      node.userData.formaSelectionOverlay ||
      !node.visible ||
      exclude.has(node.name)
    )
      return;
    const vertices = cadVertices(node);
    if (!vertices) return;
    node.updateWorldMatrix(true, false);
    for (const vertex of vertices) {
      const world = vertex.point.clone().applyMatrix4(node.matrixWorld);
      projected.copy(world).project(camera);
      if (projected.z < -1 || projected.z > 1) continue;
      const x = ((projected.x + 1) / 2) * size.width;
      const y = ((1 - projected.y) / 2) * size.height;
      const distancePx = Math.hypot(x - screen.x, y - screen.y);
      if (distancePx <= radiusPx && (!best || distancePx < best.distancePx))
        best = { bodyId: node.name, vertex, world, distancePx };
    }
  });
  return best;
}
