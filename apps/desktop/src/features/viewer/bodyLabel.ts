import type * as THREE from "three";
import { t } from "../../i18n";

export function bodyLabel(object: THREE.Object3D): string {
  const displayName = object.userData.formaDisplayName;
  if (typeof displayName === "string" && displayName.trim()) return displayName;
  const templateLabel = object.userData.formaLabel;
  return typeof templateLabel === "string" ? t(templateLabel) : object.name;
}
