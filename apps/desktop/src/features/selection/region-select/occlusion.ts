import * as THREE from "three";
import {
  prepareRegion,
  type Point2,
  type SelectionRegion,
} from "./regionGeometry";

const samplesPerBody = 48;

/**
 * Keeps only region hits that are actually visible: up to 48 triangle samples
 * of each hit body inside the region are ray-cast from the camera, and the body
 * counts when at least one ray reaches it first. A body touched by the region
 * only through slivers with no sample inside keeps its hit (no evidence of
 * occlusion). X-ray style selection skips this test entirely.
 */
export function visibleRegionHits(options: {
  object: THREE.Object3D;
  camera: THREE.Camera;
  size: { width: number; height: number };
  region: SelectionRegion;
  hits: readonly string[];
}): string[] {
  const { object, camera, size, region } = options;
  const inside = prepareRegion(region);
  const wanted = new Set(options.hits);
  const meshes = new Map<string, THREE.Mesh>();
  object.traverse((node) => {
    if (
      node instanceof THREE.Mesh &&
      wanted.has(node.name) &&
      !node.userData.formaSelectionOverlay
    )
      meshes.set(node.name, node);
  });
  const caster = new THREE.Raycaster();
  const projected = new THREE.Vector3();
  const visible: string[] = [];
  for (const id of options.hits) {
    const mesh = meshes.get(id);
    if (!mesh) continue;
    const samples = triangleSamples(mesh, camera, size, inside, projected);
    if (!samples.length) {
      visible.push(id);
      continue;
    }
    // The region's own centre covers large faces whose triangle centres lie outside it.
    samples.push(regionCentre(region));
    const seen = samples.some(([x, y]) => {
      caster.setFromCamera(
        new THREE.Vector2((x / size.width) * 2 - 1, -(y / size.height) * 2 + 1),
        camera,
      );
      const first = caster
        .intersectObject(object, true)
        .find(
          (hit) =>
            hit.object instanceof THREE.Mesh &&
            hit.object.visible &&
            !hit.object.userData.formaSelectionOverlay,
        );
      return first?.object === mesh;
    });
    if (seen) visible.push(id);
  }
  return visible;
}

function regionCentre(region: SelectionRegion): Point2 {
  if (region.kind === "circle") return [...region.center];
  const n = region.points.length;
  return [
    region.points.reduce((sum, point) => sum + point[0], 0) / n,
    region.points.reduce((sum, point) => sum + point[1], 0) / n,
  ];
}

function triangleSamples(
  mesh: THREE.Mesh,
  camera: THREE.Camera,
  size: { width: number; height: number },
  inside: (primitive: readonly [Point2]) => boolean,
  scratch: THREE.Vector3,
): Point2[] {
  const position = mesh.geometry.getAttribute("position");
  if (!position) return [];
  mesh.updateWorldMatrix(true, false);
  const index = mesh.geometry.getIndex();
  const triangles = Math.floor((index ? index.count : position.count) / 3);
  const candidates: Point2[] = [];
  const corner = (triangle: number, k: number) =>
    index ? index.getX(triangle * 3 + k) : triangle * 3 + k;
  const centroid = new THREE.Vector3();
  for (let triangle = 0; triangle < triangles; triangle++) {
    centroid.set(0, 0, 0);
    for (let k = 0; k < 3; k++)
      centroid.add(
        scratch.fromBufferAttribute(
          position as THREE.BufferAttribute,
          corner(triangle, k),
        ),
      );
    centroid
      .multiplyScalar(1 / 3)
      .applyMatrix4(mesh.matrixWorld)
      .project(camera);
    if (centroid.z < -1 || centroid.z > 1) continue;
    const point: Point2 = [
      ((centroid.x + 1) / 2) * size.width,
      ((1 - centroid.y) / 2) * size.height,
    ];
    if (inside([point])) candidates.push(point);
  }
  if (candidates.length <= samplesPerBody) return candidates;
  const step = candidates.length / samplesPerBody;
  return Array.from(
    { length: samplesPerBody },
    (_, i) => candidates[Math.floor(i * step)],
  );
}
