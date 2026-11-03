import * as THREE from "three";
import type { Vec3 } from "../frames";
import type { PointerSample } from "../transformSession";
import { cadPointToRender, renderDirectionToCad } from "./renderBasis";

export interface ScreenPoint {
  x: number;
  y: number;
}

function ray(
  camera: THREE.Camera,
  size: { width: number; height: number },
  point: ScreenPoint,
) {
  const caster = new THREE.Raycaster();
  caster.setFromCamera(
    new THREE.Vector2(
      (point.x / size.width) * 2 - 1,
      -(point.y / size.height) * 2 + 1,
    ),
    camera,
  );
  return caster.ray;
}

export function projectToScreen(
  camera: THREE.Camera,
  size: { width: number; height: number },
  pointMm: Vec3,
  frame?: THREE.Matrix4,
): ScreenPoint {
  const projected = cadPointToRender(pointMm, frame).project(camera);
  return {
    x: ((projected.x + 1) / 2) * size.width,
    y: ((1 - projected.y) / 2) * size.height,
  };
}

/**
 * Absolute pointer readings relative to the modal start. Translation lies on the
 * view plane through the pivot; rotation is the screen angle around the pivot,
 * positive counter-clockwise when the rotation axis faces the viewer.
 */
export function pointerSample(options: {
  camera: THREE.Camera;
  size: { width: number; height: number };
  start: ScreenPoint;
  current: ScreenPoint;
  pivotMm: Vec3;
  rotationAxisMm: Vec3;
  /** CAD → world frame of the displayed root; defaults to the bare render basis. */
  frame?: THREE.Matrix4;
}): PointerSample | null {
  const { camera, size, start, current, pivotMm, frame } = options;
  if (size.width <= 0 || size.height <= 0) return null;
  const pivot = cadPointToRender(pivotMm, frame);
  const facing = camera.getWorldDirection(new THREE.Vector3());
  const plane = new THREE.Plane().setFromNormalAndCoplanarPoint(facing, pivot);
  const from = ray(camera, size, start).intersectPlane(
    plane,
    new THREE.Vector3(),
  );
  const to = ray(camera, size, current).intersectPlane(
    plane,
    new THREE.Vector3(),
  );
  const translationMm: Vec3 =
    from && to ? renderDirectionToCad(to.clone().sub(from), frame) : [0, 0, 0];
  // renderDirectionToCad normalizes; restore the dragged length.
  const length = from && to ? to.distanceTo(from) : 0;
  const scaled = translationMm.map((value) => value * length) as Vec3;
  const centre = projectToScreen(camera, size, pivotMm, frame);
  const a = Math.atan2(-(start.y - centre.y), start.x - centre.x);
  const b = Math.atan2(-(current.y - centre.y), current.x - centre.x);
  let angle = ((b - a) * 180) / Math.PI;
  angle = ((((angle + 180) % 360) + 360) % 360) - 180;
  const axis = cadPointToRender(options.rotationAxisMm, frame)
    .sub(cadPointToRender([0, 0, 0], frame))
    .normalize();
  const towardViewer = axis.dot(facing) < 0 ? 1 : -1;
  const startRadius = Math.hypot(start.x - centre.x, start.y - centre.y);
  const radius = Math.hypot(current.x - centre.x, current.y - centre.y);
  return {
    translationMm: scaled.every(Number.isFinite) ? scaled : [0, 0, 0],
    angleDeg: Number.isFinite(angle) ? angle * towardViewer : 0,
    scaleDelta: startRadius > 4 ? radius / startRadius - 1 : 0,
  };
}

/** Rotation keeps winding past ±180° by tracking the previous unwrapped angle. */
export function unwrapAngle(previous: number, next: number): number {
  let delta = next - (((((previous + 180) % 360) + 360) % 360) - 180);
  if (delta > 180) delta -= 360;
  if (delta < -180) delta += 360;
  return previous + delta;
}
