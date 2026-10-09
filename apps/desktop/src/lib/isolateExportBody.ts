import * as THREE from "three";
import { t } from "../i18n";

export function isolateExportBody(object: THREE.Group, bodyId: string): THREE.Group {
  let selected: THREE.Mesh | null = null;
  object.traverse((item) => {
    if (item instanceof THREE.Mesh && item.userData.formaBodyId === bodyId) {
      selected = item;
    }
  });
  if (!selected) throw new Error(t("Selected CAD body is missing"));
  const mesh = (selected as THREE.Mesh).clone();
  (selected as THREE.Mesh).matrixWorld.decompose(
    mesh.position,
    mesh.quaternion,
    mesh.scale,
  );
  const group = new THREE.Group();
  group.add(mesh);
  return group;
}
