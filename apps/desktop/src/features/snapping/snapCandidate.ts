import * as THREE from "three";
import type { Vec3 } from "../transform/frames";
import {
  renderDirectionToCad,
  renderPointToCad,
} from "../transform/viewport/renderBasis";
import {
  faceForTriangle,
  faceReference,
  faceTriangleCounts,
} from "../viewer/faceSelection";
import { pickCadVertex } from "../viewer/vertexSelection";
import type { ExactSnapQuery } from "./snapApi";

/**
 * A snap target found on the displayed tessellation. `pointMm` is only a
 * display estimate; `query` (when present) lets OCCT compute the exact point
 * on the committed body.
 */
export interface SnapCandidate {
  kind: "vertex" | "face" | "surface";
  bodyId: string;
  pointMm: Vec3;
  query: ExactSnapQuery | null;
}

export function findSnapCandidate(options: {
  object: THREE.Object3D;
  camera: THREE.Camera;
  screen: { x: number; y: number };
  size: { width: number; height: number };
  frame: THREE.Matrix4;
  /** Bodies that must not be snapped to, e.g. the ones being moved. */
  exclude?: ReadonlySet<string>;
  vertexRadiusPx?: number;
  surfaces?: boolean;
}): SnapCandidate | null {
  const { object, camera, screen, size, frame } = options;
  const exclude = options.exclude ?? new Set<string>();
  const vertex = pickCadVertex(
    object,
    camera,
    screen,
    size,
    options.vertexRadiusPx ?? 14,
    exclude,
  );
  if (vertex)
    return {
      kind: "vertex",
      bodyId: vertex.bodyId,
      pointMm: renderPointToCad(vertex.world, frame),
      query: vertex.vertex.topologyRef
        ? { kind: "vertex", reference: vertex.vertex.topologyRef }
        : null,
    };
  if (options.surfaces === false) return null;
  const caster = new THREE.Raycaster();
  caster.setFromCamera(
    new THREE.Vector2(
      (screen.x / size.width) * 2 - 1,
      -(screen.y / size.height) * 2 + 1,
    ),
    camera,
  );
  const hit = caster
    .intersectObject(object, true)
    .find(
      (item) =>
        item.object instanceof THREE.Mesh &&
        item.object.visible &&
        !item.object.userData.formaSelectionOverlay &&
        !exclude.has(item.object.name),
    );
  if (!hit) return null;
  const mesh = hit.object as THREE.Mesh;
  const counts = faceTriangleCounts(mesh);
  const ordinal =
    counts && hit.faceIndex != null
      ? faceForTriangle(counts, hit.faceIndex)
      : null;
  const reference = ordinal ? faceReference(mesh, ordinal) : null;
  return {
    kind: reference ? "face" : "surface",
    bodyId: mesh.name,
    pointMm: renderPointToCad(hit.point, frame),
    query: reference
      ? {
          kind: "face",
          reference,
          rayOriginMm: renderPointToCad(caster.ray.origin, frame),
          rayDirection: renderDirectionToCad(caster.ray.direction, frame),
        }
      : null,
  };
}
