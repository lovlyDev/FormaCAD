import * as THREE from "three";
import type { RenderMode } from "../renderMode";

/** X-Ray explicitly bypasses the depth buffer to reveal edges inside enclosing bodies. */
export function addBodyOutlines(object: THREE.Group, mode: RenderMode, selected: string | null, light: boolean) {
  const lines: THREE.LineSegments[] = [];
  if (mode === "edges" || mode === "xray" || mode === "ghost") object.traverse(node => {
    if (!(node instanceof THREE.Mesh) || node.userData.formaSelectionOverlay) return;
    const focused = node.name === selected, xray = mode === "xray";
    const line = new THREE.LineSegments(new THREE.EdgesGeometry(node.geometry, 25), new THREE.LineBasicMaterial({
      color: xray ? (light ? "#4338ca" : "#c4b5fd") : focused && mode === "ghost" ? (light ? "#6d28d9" : "#c4b5fd") : light ? "#495667" : "#657487",
      transparent: true,
      opacity: xray ? .85 : mode === "ghost" && !focused ? .12 : .7,
      depthTest: !xray,
      depthWrite: false,
    }));
    line.userData.formaSelectionOverlay = true;
    line.userData.formaAppearanceOutline = true;
    line.raycast = () => {};
    if (xray) line.renderOrder = 20;
    node.add(line); lines.push(line);
  });
  let disposed = false;
  return () => {
    if (disposed) return;
    disposed = true;
    lines.forEach(line => { line.removeFromParent(); line.geometry.dispose(); (line.material as THREE.Material).dispose(); });
  };
}
