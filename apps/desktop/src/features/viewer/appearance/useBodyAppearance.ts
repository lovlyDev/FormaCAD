import { useEffect } from "react";
import type * as THREE from "three";
import type { RenderMode } from "../renderMode";
import { applyBodyAppearance } from "./bodyAppearance";
const none: readonly string[] = [];
export function useBodyAppearance(object: THREE.Group, mode: RenderMode, selected: string | null, light: boolean, also: readonly string[] = none) {
  useEffect(() => applyBodyAppearance(object, mode, selected, light, also), [object, mode, selected, light, also]);
}
