import * as THREE from "three";
import { finiteBox } from "../model-preview/cameraFit";
import type { OrbitHandle } from "./CameraControl";

/** Apply the complete framing before the next input event, without a queued zoom. */
export function fitOrthographicBox(camera: THREE.OrthographicCamera, controls: OrbitHandle, box: THREE.Box3, margin = 1.8): boolean {
  if (!finiteBox(box) || !Number.isFinite(margin) || margin <= 0) return false;
  const center = box.getCenter(new THREE.Vector3());
  camera.position.add(center.clone().sub(controls.target));
  controls.target.copy(center);
  camera.lookAt(center);
  camera.updateMatrixWorld(true);
  let halfWidth = 0, halfHeight = 0;
  for (const x of [box.min.x, box.max.x]) for (const y of [box.min.y, box.max.y]) for (const z of [box.min.z, box.max.z]) {
    const point = new THREE.Vector3(x, y, z).applyMatrix4(camera.matrixWorldInverse);
    halfWidth = Math.max(halfWidth, Math.abs(point.x));
    halfHeight = Math.max(halfHeight, Math.abs(point.y));
  }
  camera.zoom = Math.min((camera.right-camera.left)/(2*Math.max(halfWidth,0.01)), (camera.top-camera.bottom)/(2*Math.max(halfHeight,0.01)))/margin;
  camera.updateProjectionMatrix();
  controls.update();
  return true;
}
