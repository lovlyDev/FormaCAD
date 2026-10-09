import { useEffect, useRef, type RefObject } from "react";
import { useFrame, useThree } from "@react-three/fiber";
import { useBounds } from "@react-three/drei";
import * as THREE from "three";
import { persist, readState } from "../../lib/persistence";
export function CameraControl({
  preset,
  fit,
  reset,
  object,
  storageKey,
  controlsRef,
  appliedPreset,
}: {
  preset: string;
  fit: number;
  reset: number;
  object: THREE.Group;
  storageKey: string;
  controlsRef: RefObject<OrbitHandle | null>;
  appliedPreset: RefObject<string>;
}) {
  const bounds = useBounds();
  const camera = useThree((state) => state.camera);
  const last = useRef<{ fit: number; reset: number; preset: string; camera: THREE.Camera } | null>(null);
  const api = useRef(bounds);
  useEffect(() => {
    api.current = bounds;
  }, [bounds]);
  useEffect(() => {
    const controls = controlsRef.current;
    if (!controls) return;
    const previous = last.current;
    last.current = { fit, reset, preset, camera };
    const saved = readState<CameraSnapshot | null>(storageKey, null);
    if ((!previous || (previous.fit === fit && previous.reset === reset && previous.preset === preset && previous.camera === camera)) && validCamera(saved)) {
      api.current.refresh(object).clip();
      camera.position.fromArray(saved.position);
      camera.quaternion.fromArray(saved.quaternion);
      camera.up.fromArray(saved.up);
      controls.target.fromArray(saved.target);
      if (camera instanceof THREE.PerspectiveCamera || camera instanceof THREE.OrthographicCamera) {
        camera.zoom = saved.zoom;
        camera.updateProjectionMatrix();
      }
      controls.update();
      return;
    }
    if (!previous || previous.fit !== fit || previous.reset !== reset || previous.camera !== camera) api.current.refresh(object).clip().fit();
    if (previous && previous.reset === reset) return;
    const positions: Record<string, [number, number, number]> = {
      iso: [180, 140, 180],
      top: [0, 260, 0.001],
      front: [0, 40, 260],
      side: [260, 40, 0],
    };
    const framing = api.current.refresh(object).clip();
    const { center } = framing.getSize();
    const distance = Math.max(camera.position.distanceTo(center), 1);
    const direction = new THREE.Vector3(
      ...(positions[preset] ?? positions.iso),
    ).normalize();
    framing
      .moveTo(center.clone().addScaledVector(direction, distance))
      .lookAt({ target: center, up: [0, 1, 0] });
    if (camera instanceof THREE.OrthographicCamera) framing.fit();
  }, [preset, object, reset, camera, controlsRef, fit, storageKey]);
  useFrame(() => {
    const controls = controlsRef.current;
    if (!controls || !last.current) return;
    if (appliedPreset.current !== preset) {
      const directions: Record<string, [number, number, number]> = {
        iso: [1, 0.8, 1],
        top: [0, 1, 0.00001],
        front: [0, 0, 1],
        side: [1, 0, 0],
      };
      const distance = Math.max(camera.position.distanceTo(controls.target), 1);
      const direction = new THREE.Vector3(
        ...(directions[preset] ?? directions.iso),
      ).normalize();
      const target = controls.target.clone();
      api.current.refresh(object).clip();
      camera.position.copy(target).addScaledVector(direction, distance);
      camera.up.set(0, preset === "top" ? 0 : 1, preset === "top" ? -1 : 0);
      camera.lookAt(target);
      controls.update();
      appliedPreset.current = preset;
    }
    const snapshot = JSON.stringify({
      position: camera.position.toArray().map(roundCamera),
      quaternion: camera.quaternion.toArray().map(roundCamera),
      up: camera.up.toArray().map(roundCamera),
      target: controls.target.toArray().map(roundCamera),
      zoom: roundCamera((camera as THREE.PerspectiveCamera).zoom),
    });
    if (localStorage.getItem(storageKey) !== snapshot) persist(storageKey, snapshot);
  });
  return null;
}
type CameraSnapshot = { position: number[]; quaternion: number[]; up: number[]; target: number[]; zoom: number };
export type OrbitHandle = { enabled: boolean; target: THREE.Vector3; update: () => void };
const roundCamera = (value: number) => Math.round(value * 100_000) / 100_000;
function validCamera(value: CameraSnapshot | null): value is CameraSnapshot {
  return !!value && [value.position, value.up, value.target].every(v => Array.isArray(v) && v.length === 3 && v.every(Number.isFinite)) && Array.isArray(value.quaternion) && value.quaternion.length === 4 && value.quaternion.every(Number.isFinite) && Number.isFinite(value.zoom) && value.zoom > 0;
}
