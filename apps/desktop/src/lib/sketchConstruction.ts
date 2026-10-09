import type { SketchOperation } from "./sketchDocument";
import { retainUsedGeometry } from "./sketchGeometry";

/** Guides share stable sketch point IDs, so their constraints can drive the profile. */
export function addConstructionLine(operation: SketchOperation, startPointId: string, endPointId: string): SketchOperation | null {
  if (startPointId === endPointId || operation.lines.length >= 64
    || !operation.points.some((point) => point.id === startPointId)
    || !operation.points.some((point) => point.id === endPointId)) return null;
  const ids = new Set([...operation.points, ...operation.lines, ...operation.constraints].map((item) => item.id));
  let index = 1;
  while (ids.has(`construction_${index}`)) index += 1;
  return { ...operation, lines: [...operation.lines, {
    id: `construction_${index}`, startPointId, endPointId, construction: true,
  }] };
}

/** Delete only the guide and its own constraints; preserve all shared profile points. */
export function removeConstructionLine(operation: SketchOperation, lineId: string): SketchOperation {
  if (!operation.lines.some((line) => line.id === lineId && line.construction)) return operation;
  const lines = operation.lines.filter((line) => line.id !== lineId);
  return retainUsedGeometry({ ...operation, lines });
}
