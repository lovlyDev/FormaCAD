import * as THREE from "three";
import { cadEdges } from "./edgeSelection";
import type { EdgeTopologyReference } from "./topology/topologyReference";
import { clipSegment, getSectionPlane } from "./sections/sectionClipping";
interface EdgeHit { bodyId: string; edgeOrdinal: number; semanticKey: string | null; topologyRef?: EdgeTopologyReference; point: THREE.Vector3; distancePx: number; depth: number }
/** Pixel tolerance is independent of model units, projection and zoom. Runs on click only. */
export function pickCadEdge(root: THREE.Object3D, camera: THREE.Camera, cursor: { x: number; y: number }, width: number, height: number, tolerancePx = 8): EdgeHit | null {
  if (width <= 0 || height <= 0) return null;
  root.updateWorldMatrix(true, true);
  camera.updateWorldMatrix(true, false);
  const surfaces: THREE.Mesh[] = [];
  const candidates: EdgeHit[] = [];
  root.traverse(node => {
    if (!(node instanceof THREE.Mesh) || node.userData.formaSelectionOverlay || !node.visible) return;
    surfaces.push(node);
    cadEdges(node)?.forEach((edge, index) => {
      let best: EdgeHit | null = null;
      for (let i = 1; i < edge.points.length; i++) {
        const a = new THREE.Vector3(...edge.points[i - 1]).applyMatrix4(node.matrixWorld);
        const b = new THREE.Vector3(...edge.points[i]).applyMatrix4(node.matrixWorld);
        if (!clipSegment(a, b, getSectionPlane(node))) continue;
        const pa = a.clone().project(camera), pb = b.clone().project(camera);
        if (pa.z < -1 || pa.z > 1 || pb.z < -1 || pb.z > 1) continue;
        const ax = (pa.x + 1) * width / 2, ay = (1 - pa.y) * height / 2;
        const dx = (pb.x - pa.x) * width / 2, dy = (pa.y - pb.y) * height / 2;
        const length = dx * dx + dy * dy;
        const t = length ? THREE.MathUtils.clamp(((cursor.x - ax) * dx + (cursor.y - ay) * dy) / length, 0, 1) : 0;
        const distancePx = Math.hypot(cursor.x - ax - t * dx, cursor.y - ay - t * dy);
        if (distancePx > tolerancePx || (best && best.distancePx <= distancePx)) continue;
        // Screen interpolation is perspective correct after unprojecting onto the world segment.
        const screen = new THREE.Vector3(pa.x + t * (pb.x - pa.x), pa.y + t * (pb.y - pa.y), 0);
        const ray = new THREE.Raycaster(); ray.setFromCamera(new THREE.Vector2(screen.x, screen.y), camera);
        const point = new THREE.Vector3();
        ray.ray.distanceSqToSegment(a, b, undefined, point);
        best = { bodyId: node.name, edgeOrdinal: index + 1, semanticKey: edge.semanticKey, ...(edge.topologyRef ? { topologyRef: edge.topologyRef } : {}), point, distancePx, depth: point.clone().project(camera).z };
      }
      if (best) candidates.push(best);
    });
  });
  candidates.sort((a, b) => a.distancePx - b.distancePx || a.depth - b.depth);
  const epsilon = Math.max(new THREE.Box3().setFromObject(root).getSize(new THREE.Vector3()).length() * 1e-4, 1e-6);
  for (const hit of candidates) {
    const projected = hit.point.clone().project(camera);
    const ray = new THREE.Raycaster(); ray.setFromCamera(new THREE.Vector2(projected.x, projected.y), camera);
    const surface = ray.intersectObjects(surfaces, false)[0];
    if (!surface || surface.distance + epsilon >= ray.ray.origin.distanceTo(hit.point)) return hit;
  }
  return null;
}
