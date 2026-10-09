import * as THREE from "three";
import type { RenderMode } from "../renderMode";
import { getSectionPlane } from "../sections/sectionClipping";
const sources = new WeakMap<THREE.Material, THREE.Material>();

/** Export clones share material references: replace display clones before any exporter awaits. */
export function restoreExportAppearance(object: THREE.Object3D) {
  object.traverse(node => {
    if (!(node instanceof THREE.Mesh)) return;
    const source = (material: THREE.Material) => sources.get(material) ?? material;
    node.material = Array.isArray(node.material) ? node.material.map(source) : source(node.material);
  });
}

/** Display materials are owned clones. Source materials and geometry stay untouched. */
export function applyBodyAppearance(object: THREE.Group, mode: RenderMode, selected: string | null, light: boolean) {
  const owned: { mesh: THREE.Mesh; original: THREE.Material | THREE.Material[]; clones: THREE.Material[]; order: number }[] = [];
  object.traverse(node => {
    if (!(node instanceof THREE.Mesh) || node.userData.formaSelectionOverlay) return;
    const original = node.material, order = node.renderOrder;
    const focused = node.name === selected;
    const clones = (Array.isArray(original) ? original : [original]).map((source: THREE.Material) => {
      const material = source.clone();
      // SectionScene owns this temporary plane. Inheriting it would make it the
      // clone's restoration baseline when a section outlives a Model remount.
      const section = getSectionPlane(node);
      if (section && source.clippingPlanes?.includes(section)) {
        const persistent = source.clippingPlanes.filter(plane => plane !== section);
        material.clippingPlanes = persistent.length ? persistent.map(plane => plane.clone()) : null;
      }
      sources.set(material, source);
      material.transparent = mode === "transparent" || mode === "xray" || (mode === "ghost" && !focused);
      material.opacity = mode === "transparent" ? .35 : mode === "xray" ? .12 : mode === "ghost" && !focused ? .09 : 1;
      material.depthWrite = !material.transparent;
      if (mode === "xray") material.side = THREE.DoubleSide;
      if (material instanceof THREE.MeshStandardMaterial) {
        material.wireframe = mode === "wireframe";
        if (focused) material.color.set(light ? "#7c3aed" : "#a78bfa");
      }
      return material;
    });
    node.material = Array.isArray(original) ? clones : clones[0];
    owned.push({ mesh: node, original, clones, order });
  });
  let disposed = false;
  return () => {
    if (disposed) return;
    disposed = true;
    for (const { mesh, original, clones, order } of owned) {
      mesh.material = original;
      mesh.renderOrder = order;
      clones.forEach(material => { sources.delete(material); material.dispose(); });
    }
  };
}
