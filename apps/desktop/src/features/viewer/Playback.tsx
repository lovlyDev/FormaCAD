import { useEffect, useMemo, useRef } from "react";
import { useFrame, useThree } from "@react-three/fiber";
import * as THREE from "three";
import { persist, readState } from "../../lib/persistence";

export function SmoothZoom() {
  const { gl, camera } = useThree();
  const controls = useThree((s) => s.controls) as unknown as
    | {
        target: THREE.Vector3;
        minDistance: number;
        maxDistance: number;
        update: () => void;
      }
    | undefined;
  const pending = useRef(0);
  useEffect(() => {
    const canvas = gl.domElement;
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      const pixels =
        event.deltaY *
        (event.deltaMode === 1
          ? 16
          : event.deltaMode === 2
            ? canvas.clientHeight
            : 1);
      pending.current = THREE.MathUtils.clamp(
        pending.current + THREE.MathUtils.clamp(pixels, -240, 240) * 0.0015,
        -1.5,
        1.5,
      );
    };
    const stop = () => {
      pending.current = 0;
    };
    canvas.addEventListener("wheel", wheel, { passive: false });
    canvas.addEventListener("pointerdown", stop);
    return () => {
      canvas.removeEventListener("wheel", wheel);
      canvas.removeEventListener("pointerdown", stop);
    };
  }, [gl]);
  useFrame((_, delta) => {
    if (!controls || Math.abs(pending.current) < 0.00001) return;
    const step = pending.current * (1 - Math.exp(-10 * Math.min(delta, 0.05)));
    pending.current -= step;
    if (camera instanceof THREE.OrthographicCamera) {
      camera.zoom = THREE.MathUtils.clamp(
        camera.zoom * Math.exp(-step),
        0.02,
        5000,
      );
      camera.updateProjectionMatrix();
    } else {
      const offset = camera.position.clone().sub(controls.target);
      const distance = THREE.MathUtils.clamp(
        offset.length() * Math.exp(step),
        controls.minDistance,
        controls.maxDistance,
      );
      camera.position.copy(controls.target).add(offset.setLength(distance));
    }
    controls.update();
  });
  return null;
}

export function hasMotion(object: THREE.Object3D) {
  let found = false;
  object.traverse((node) => {
    if (node.userData.formaMotion?.speed) found = true;
  });
  return found;
}

export function Playback({
  object,
  playing,
  reset,
  storageKey,
}: {
  object: THREE.Group;
  playing: boolean;
  reset: number;
  storageKey?: string;
}) {
  const elapsed = useRef(0);
  const bindings = useMemo(() => {
    const result: {
      node: THREE.Object3D;
      axis: THREE.Vector3;
      speed: number;
      initial: THREE.Quaternion;
    }[] = [];
    object.traverse((node) => {
      const data = node.userData.formaMotion;
      if (
        !data ||
        !Array.isArray(data.axis) ||
        data.axis.length !== 3 ||
        !data.axis.every(Number.isFinite) ||
        !Number.isFinite(data.speed) ||
        Math.abs(data.speed) > 7200
      )
        return;
      const axis = new THREE.Vector3(
        ...(data.axis as [number, number, number]),
      );
      if (axis.lengthSq() < 0.000001) return;
      result.push({
        node,
        axis: axis.normalize(),
        speed: THREE.MathUtils.degToRad(data.speed),
        initial: node.quaternion.clone(),
      });
    });
    return result;
  }, [object]);
  useEffect(() => {
    elapsed.current = reset > 0 || !storageKey ? 0 : readState(storageKey, 0);
    if (!Number.isFinite(elapsed.current)) elapsed.current = 0;
    bindings.forEach(({ node, initial, axis, speed }) => node.quaternion.copy(initial).multiply(new THREE.Quaternion().setFromAxisAngle(axis, elapsed.current * speed)));
    return () => {
      bindings.forEach(({ node, initial }) => node.quaternion.copy(initial));
    };
  }, [bindings, reset, storageKey]);
  const rotation = useMemo(() => new THREE.Quaternion(), []);
  useFrame((_, delta) => {
    if (!playing) return;
    elapsed.current += Math.min(delta, 0.05);
    if (storageKey) persist(storageKey, JSON.stringify(elapsed.current));
    bindings.forEach(({ node, axis, speed, initial }) => {
      node.quaternion
        .copy(initial)
        .multiply(rotation.setFromAxisAngle(axis, elapsed.current * speed));
      if (import.meta.env.DEV)
        node.userData.playbackAngle = elapsed.current * speed;
    });
  });
  return null;
}
