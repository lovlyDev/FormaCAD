import type { SelectionKind, SelectionTarget } from "../selectionModel";
import {
  prepareRegion,
  type ScreenPrimitive,
  type SelectionRegion,
} from "./regionGeometry";
export interface ProjectedTarget {
  target: SelectionTarget;
  primitives: readonly ScreenPrimitive[];
  inFrontOfCamera: boolean;
  occluded: boolean;
}
/** Projection and occlusion are supplied by the renderer for the same captured scene. */
export function regionSelection(
  region: SelectionRegion,
  candidates: readonly ProjectedTarget[],
  filter: SelectionKind,
  through: boolean,
): string[] {
  const hits = new Set<string>();
  const intersects = prepareRegion(region);
  for (const candidate of candidates) {
    const { target } = candidate;
    if (
      target.kind !== filter ||
      !target.selectable ||
      !target.visible ||
      !candidate.inFrontOfCamera ||
      (!through && candidate.occluded) ||
      hits.has(target.key)
    )
      continue;
    if (candidate.primitives.some(intersects)) hits.add(target.key);
  }
  return [...hits];
}
