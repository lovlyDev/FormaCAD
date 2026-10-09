import { useEffect, type RefObject } from "react";
import { useFrame, useThree } from "@react-three/fiber";
import * as THREE from "three";
import type { OrbitHandle } from "./CameraControl";
export function Capture({ onReady, controlsRef }: { onReady: (fn: () => string) => void; controlsRef: RefObject<OrbitHandle | null> }) {
  const { gl, scene, camera } = useThree();
  useFrame(() => {
    if (import.meta.env.DEV) {
      gl.domElement.dataset.camera = JSON.stringify({
        position: camera.position.toArray(),
        target: controlsRef.current?.target.toArray() ?? [0, 0, 0],
        zoom: camera.zoom,
        quaternion: camera.quaternion.toArray(),
      });
      gl.domElement.dataset.background = scene.background instanceof THREE.Color
        ? `#${scene.background.getHexString()}` : "none";
    }
    if (import.meta.env.DEV) {
      const motion: { name: string; speed: number; rotation: number[] }[] = [];
      scene.traverse((node) => {
        if (node.userData.formaMotion)
          motion.push({
            name: node.name,
            speed: node.userData.formaMotion.speed,
            rotation: node.quaternion.toArray(),
          });
      });
      gl.domElement.dataset.motion = JSON.stringify(motion);
    }
  });
  useEffect(() => {
    onReady(() => {
      gl.render(scene, camera);
      return gl.domElement.toDataURL("image/png");
    });
  }, [gl, scene, camera, onReady]);
  return null;
}
