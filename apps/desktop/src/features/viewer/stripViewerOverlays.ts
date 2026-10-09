import * as THREE from "three";

export function stripViewerOverlays(object: THREE.Object3D): void {
  const overlays: THREE.Object3D[] = [];
  object.traverse((node) => {
    if (node.userData.formaSelectionOverlay) overlays.push(node);
  });
  overlays.forEach((node) => node.removeFromParent());
}
