import * as THREE from "three";

/** Validated `viewport.orbit` request as sent by the host (camelCase). */
export interface OrbitSpec {
  width: number;
  height: number;
  fps: number;
  durationMs: number;
  turns: number;
  clockwise: boolean;
  up: "x" | "y" | "z" | "-x" | "-y" | "-z";
  tiltDeg: number;
  elevationDeg: number;
  startDeg: number;
  zoom: number;
  background: "theme" | "light" | "dark";
  format: "mp4" | "webm";
  bitrateKbps: number;
}

export const CAMERA_FOV = 32;
const DEG = Math.PI / 180;

export const frameCount = (spec: OrbitSpec) => Math.ceil((spec.durationMs * spec.fps) / 1000);

/** Turntable angle (radians, about the vertical axis) of frame `index`. */
export function turntableAngle(spec: OrbitSpec, index: number): number {
  const progress = index / frameCount(spec);
  const sign = spec.clockwise ? -1 : 1;
  return (spec.startDeg + sign * 360 * spec.turns * progress) * DEG;
}

/** The viewer scene is Y-up; CAD Z maps to +Y and CAD Y to −Z (glTF convention). */
const cadAxis: Record<string, THREE.Vector3> = {
  x: new THREE.Vector3(1, 0, 0),
  y: new THREE.Vector3(0, 0, -1),
  z: new THREE.Vector3(0, 1, 0),
};

function axisOf(up: OrbitSpec["up"]): THREE.Vector3 {
  const axis = cadAxis[up.replace("-", "")]!.clone();
  return up.startsWith("-") ? axis.negate() : axis;
}

/** Part orientation on the turntable: chosen up axis, then tilt toward the camera. */
export function standQuaternion(spec: OrbitSpec): THREE.Quaternion {
  const stand = new THREE.Quaternion().setFromUnitVectors(axisOf(spec.up), new THREE.Vector3(0, 1, 0));
  const tilt = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(1, 0, 0), spec.tiltDeg * DEG);
  return tilt.multiply(stand);
}

/** Full model rotation for one frame: stand first, then spin about the vertical. */
export function frameQuaternion(spec: OrbitSpec, index: number): THREE.Quaternion {
  const spin = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), turntableAngle(spec, index));
  return spin.multiply(standQuaternion(spec));
}

/**
 * Camera position for a bounding sphere of `radius` around the origin: the
 * sphere fits the narrower field of view at zoom 1 for every turntable angle.
 */
export function cameraPosition(spec: OrbitSpec, radius: number): THREE.Vector3 {
  const vertical = (CAMERA_FOV * DEG) / 2;
  const horizontal = Math.atan(Math.tan(vertical) * (spec.width / spec.height));
  const distance = (Math.max(radius, 1e-3) * 1.08) / Math.sin(Math.min(vertical, horizontal)) / spec.zoom;
  const elevation = spec.elevationDeg * DEG;
  return new THREE.Vector3(0, Math.sin(elevation), Math.cos(elevation)).multiplyScalar(distance);
}

/** Bitrate in bits per second; automatic values follow the pixel rate. */
export function bitrate(spec: OrbitSpec): number {
  if (spec.bitrateKbps > 0) return spec.bitrateKbps * 1000;
  const pixelsPerSecond = spec.width * spec.height * Math.min(spec.fps, 60);
  return Math.round(Math.min(Math.max(pixelsPerSecond * 0.12, 1_000_000), 80_000_000));
}
