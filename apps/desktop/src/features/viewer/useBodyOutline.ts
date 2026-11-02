import { useEffect } from "react";
import type * as THREE from "three";
import type { RenderMode } from "./renderMode";
import { addBodyOutlines } from "./appearance/bodyOutlines";
const none: readonly string[] = [];
export function useBodyOutline(object: THREE.Group, mode: RenderMode, selected: string | null, light: boolean, also: readonly string[] = none) {
  useEffect(() => addBodyOutlines(object, mode, selected, light, also), [object, mode, selected, light, also]);
}
