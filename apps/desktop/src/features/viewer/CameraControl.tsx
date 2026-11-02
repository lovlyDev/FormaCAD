import { useEffect, useRef, type RefObject } from "react";
import { useFrame, useThree } from "@react-three/fiber";
import { useBounds } from "@react-three/drei";
import * as THREE from "three";
import { persist, readState } from "../../lib/persistence";
import { standardView, standardViewPose } from "./view-commands/standardViews";
export function CameraControl({
  preset,
  fit,
  reset,
  object,
  storageKey,
  controlsRef,
  appliedPreset,
  fitBox = null,
}: {
  preset: string;
  fit: number;
  reset: number;
  object: THREE.Group;
  storageKey: string;
  controlsRef: RefObject<OrbitHandle | null>;
  appliedPreset: RefObject<string>;
  /** Selection bounds for "frame selected"; null frames the whole model. */
  fitBox?: THREE.Box3 | null;
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
    if (!previous || previous.fit !== fit || previous.reset !== reset || previous.camera !== camera) api.current.refresh(previous && previous.fit !== fit && fitBox ? fitBox : object).clip().fit();
    if (previous && previous.reset === reset) return;
    const framing = api.current.refresh(object).clip();
    const { center } = framing.getSize();
    const distance = Math.max(camera.position.distanceTo(center), 1);
    const pose = standardViewPose(standardView(preset), center, distance);
    if (!pose) return;
    framing
      .moveTo(pose.position)
      .lookAt({ target: center, up: pose.up.toArray() });
    if (camera instanceof THREE.OrthographicCamera) framing.fit();
  }, [preset, object, reset, camera, controlsRef, fit, fitBox, storageKey]);
  useFrame(() => {
    const controls = controlsRef.current;
    if (!controls || !last.current) return;
    if (appliedPreset.current !== preset) {
      const distance = Math.max(camera.position.distanceTo(controls.target), 1);
      const pose = standardViewPose(
        standardView(preset),
        controls.target.clone(),
        distance,
      );
      api.current.refresh(object).clip();
      if (pose) {
        camera.position.copy(pose.position);
        camera.up.copy(pose.up);
        camera.lookAt(pose.target);
      }
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
