/** Read-only scene probe loaded into the running desktop app for manual acceptance runs. */
import * as Fiber from "@react-three/fiber";
import * as THREE from "three";
import { cadVertices } from "../src/features/viewer/vertexSelection";

function live() {
  const canvas = document.querySelector(
    ".viewer-area canvas",
  ) as HTMLCanvasElement | null;
  return canvas ? Fiber._roots.get(canvas)?.store.getState() : null;
}
function toScreen(world: THREE.Vector3) {
  const state = live()!;
  const point = world.clone().project(state.camera);
  const rect = state.gl.domElement.getBoundingClientRect();
  return {
    x: rect.left + ((point.x + 1) / 2) * rect.width,
    y: rect.top + ((1 - point.y) / 2) * rect.height,
  };
}
export function bodyScreen(name: string) {
  const body = live()?.scene.getObjectByName(name);
  if (!body) return null;
  return toScreen(
    new THREE.Box3().setFromObject(body).getCenter(new THREE.Vector3()),
  );
}
export function vertexScreen(name: string, role: string) {
  const body = live()?.scene.getObjectByName(name) as THREE.Mesh | undefined;
  if (!body) return null;
  const vertex = cadVertices(body)?.find(
    (item) => item.topologyRef?.role === role,
  );
  if (!vertex) return null;
  body.updateWorldMatrix(true, false);
  return toScreen(vertex.point.clone().applyMatrix4(body.matrixWorld));
}
export function bodies() {
  const names: string[] = [];
  live()?.scene.traverse((node) => {
    if (node instanceof THREE.Mesh && !node.userData.formaSelectionOverlay)
      names.push(node.name);
  });
  return names;
}
export function camera() {
  const state = live();
  if (!state) return null;
  const round = (value: number) => Math.round(value * 1000) / 1000;
  return {
    position: state.camera.position.toArray().map(round),
    quaternion: state.camera.quaternion.toArray().map(round),
    uuid: state.camera.uuid,
  };
}
