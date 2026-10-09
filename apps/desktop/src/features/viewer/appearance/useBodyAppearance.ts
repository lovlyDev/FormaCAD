import { useEffect } from "react";
import type * as THREE from "three";
import type { RenderMode } from "../renderMode";
import { applyBodyAppearance } from "./bodyAppearance";
export function useBodyAppearance(object: THREE.Group, mode: RenderMode, selected: string | null, light: boolean) {
  useEffect(() => applyBodyAppearance(object, mode, selected, light), [object, mode, selected, light]);
}
