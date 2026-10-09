import { useEffect } from "react";
import type * as THREE from "three";
import type { RenderMode } from "./renderMode";
import { addBodyOutlines } from "./appearance/bodyOutlines";
export function useBodyOutline(object: THREE.Group, mode: RenderMode, selected: string | null, light: boolean) {
  useEffect(() => addBodyOutlines(object, mode, selected, light), [object, mode, selected, light]);
}
