import { useEffect } from "react";
import { useThree } from "@react-three/fiber";
import * as THREE from "three";
export function OrbitSelection({
  object,
  selected,
  enabled,
}: {
  object: THREE.Group;
  selected: string | null;
  enabled: boolean;
}) {
  const { camera, controls } = useThree();
  useEffect(() => {
    if (!enabled || !selected || !controls) return;
    const orbit = controls as unknown as {
      target: THREE.Vector3;
      update: () => void;
    };
    const node = object.getObjectByName(selected);
    if (!node) return;
    const box = new THREE.Box3().setFromObject(node);
    if (box.isEmpty()) return;
    const target = box.getCenter(new THREE.Vector3()),
      offset = camera.position.clone().sub(orbit.target);
    orbit.target.copy(target);
    camera.position.copy(target).add(offset);
    orbit.update();
  }, [object, selected, enabled, camera, controls]);
  return null;
}
