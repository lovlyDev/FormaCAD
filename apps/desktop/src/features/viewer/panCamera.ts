import * as THREE from "three";
import type { OrbitHandle } from "./CameraControl";
/** Screen-space translation from relative mouse motion, independent of cursor coordinates. */
export function panCamera(camera: THREE.Camera, controls: OrbitHandle, dx: number, dy: number, width: number, height: number, speed = 0.3): void {
  if (![dx,dy,width,height,speed].every(Number.isFinite) || width <= 0 || height <= 0) return;
  camera.updateMatrixWorld();
  let xScale: number, yScale: number;
  if (camera instanceof THREE.PerspectiveCamera) {
    const span = 2 * camera.position.distanceTo(controls.target) * Math.tan(THREE.MathUtils.degToRad(camera.fov) / 2) / camera.zoom;
    xScale = span / height; yScale = xScale;
  } else if (camera instanceof THREE.OrthographicCamera) {
    xScale = (camera.right-camera.left) / camera.zoom / width;
    yScale = (camera.top-camera.bottom) / camera.zoom / height;
  } else return;
  const offset = new THREE.Vector3().setFromMatrixColumn(camera.matrixWorld,0).multiplyScalar(-dx*xScale*speed);
  offset.addScaledVector(new THREE.Vector3().setFromMatrixColumn(camera.matrixWorld,1),dy*yScale*speed);
  camera.position.add(offset); controls.target.add(offset); controls.update();
}
