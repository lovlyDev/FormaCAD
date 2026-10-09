import { useEffect, useMemo, useRef } from "react";
import { useFrame, useThree } from "@react-three/fiber";
import * as THREE from "three";
import { cadToWorld, clipPlane, type SectionPlane } from "./sectionPlane";
import { setSectionPlane } from "./sectionClipping";
import type { SectionReport } from "./sectionApi";

export function SectionScene({ object, plane, enabled, report, light }: { object: THREE.Group; plane: SectionPlane; enabled: boolean; report?: SectionReport; light: boolean }) {
  const { gl } = useThree(); const update = useRef<(() => void) | null>(null);
  const clipping = useMemo(() => clipPlane(plane), [plane]);
  useEffect(() => {
    if (!enabled) return;
    const previous = gl.localClippingEnabled; gl.localClippingEnabled = true;
    const materials = new Map<THREE.Material, THREE.Plane[] | null>();
    const raycasters = new Map<THREE.Mesh, THREE.Mesh["raycast"]>();
    const apply = () => {
      const liveMaterials = new Set<THREE.Material>();
      const liveMeshes = new Set<THREE.Mesh>();
      object.traverse(node => {
      if (!(node instanceof THREE.Mesh) && !(node instanceof THREE.Line)) return;
      const list = Array.isArray(node.material) ? node.material : [node.material];
      for (const material of list) {
        liveMaterials.add(material);
        if (materials.has(material)) continue;
        materials.set(material, material.clippingPlanes); material.clippingPlanes = [clipping]; material.needsUpdate = true;
      }
      if (node instanceof THREE.Mesh) liveMeshes.add(node);
      if (node instanceof THREE.Mesh && !raycasters.has(node)) {
        setSectionPlane(node, clipping);
        const original = node.raycast; raycasters.set(node, original);
        node.raycast = function (raycaster, hits) {
          const intersections: THREE.Intersection[] = []; original.call(this, raycaster, intersections);
          hits.push(...intersections.filter(hit => clipping.distanceToPoint(hit.point) >= -1e-7));
        };
      }
      });
      for (const [material, previousPlanes] of materials) {
        if (liveMaterials.has(material)) continue;
        material.clippingPlanes = previousPlanes; material.needsUpdate = true; materials.delete(material);
      }
      for (const [mesh, raycast] of raycasters) {
        if (liveMeshes.has(mesh)) continue;
        mesh.raycast = raycast; setSectionPlane(mesh, null); raycasters.delete(mesh);
      }
    };
    apply(); update.current = apply;
    return () => {
      update.current = null; gl.localClippingEnabled = previous;
      for (const [material, previousPlanes] of materials) { material.clippingPlanes = previousPlanes; material.needsUpdate = true; }
      for (const [mesh, raycast] of raycasters) { mesh.raycast = raycast; setSectionPlane(mesh, null); }
    };
  }, [object, clipping, enabled, gl]);
  useFrame(() => update.current?.());
  const geometry = useMemo(() => {
    const points: number[] = [];
    for (const curve of report?.geometry.curves ?? []) {
      for (let i = 1; i < curve.pointsMm.length; i++) points.push(...cadToWorld(curve.pointsMm[i - 1]), ...cadToWorld(curve.pointsMm[i]));
      if (curve.closed && curve.pointsMm.length > 2) points.push(...cadToWorld(curve.pointsMm[curve.pointsMm.length - 1]), ...cadToWorld(curve.pointsMm[0]));
    }
    return new THREE.BufferGeometry().setAttribute("position", new THREE.Float32BufferAttribute(points, 3));
  }, [report]);
  useEffect(() => () => geometry.dispose(), [geometry]);
  if (!enabled || !report) return null;
  return <lineSegments geometry={geometry} raycast={() => {}} renderOrder={30}><lineBasicMaterial color={light ? "#6d28d9" : "#c4b5fd"} depthTest={false} /></lineSegments>;
}
