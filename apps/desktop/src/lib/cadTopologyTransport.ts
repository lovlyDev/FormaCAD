import * as THREE from "three";
import { cadEdges } from "../features/viewer/edgeSelection";
import { faceTriangleCounts } from "../features/viewer/faceSelection";
export function preserveCadTopology(mesh: THREE.Mesh) {
  if (!cadEdges(mesh) || !faceTriangleCounts(mesh)) return;
  mesh.userData.formaCadTopology = {
    edges: mesh.geometry.userData.formaEdges,
    faceTriangleCounts: mesh.geometry.userData.formaFaceTriangleCounts,
    faceAreasMm2: mesh.geometry.userData.formaFaceAreasMm2,
    faceReferences: mesh.geometry.userData.formaFaceReferences,
  };
}
export function restoreCadTopology(mesh: THREE.Mesh) {
  const topology = mesh.userData.formaCadTopology;
  if (!topology || typeof topology !== "object" || mesh.geometry.userData.formaEdges) return;
  mesh.geometry.userData.formaEdges = topology.edges;
  mesh.geometry.userData.formaFaceTriangleCounts = topology.faceTriangleCounts;
  mesh.geometry.userData.formaFaceAreasMm2 = topology.faceAreasMm2;
  mesh.geometry.userData.formaFaceReferences = topology.faceReferences;
}
export function hasCadTopology(root: THREE.Object3D) {
  let valid = false;
  root.traverse(node => { if (node instanceof THREE.Mesh && !node.userData.formaSelectionOverlay && cadEdges(node) && faceTriangleCounts(node)) valid = true; });
  return valid;
}
