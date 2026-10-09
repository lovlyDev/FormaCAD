import type { SketchOperation } from "./sketchDocument";
export function sketchId(operation: SketchOperation, prefix: string): string {
  const ids = new Set([...operation.points, ...operation.lines, ...operation.constraints, ...(operation.bindings??[])].map((item) => item.id));
  let index = 1;
  while (ids.has(`${prefix}_${index}`)) index += 1;
  return `${prefix}_${index}`;
}
export function retainUsedGeometry(operation: SketchOperation): SketchOperation {
  const used = new Set(operation.lines.flatMap((line) => [line.startPointId, line.endPointId]));
  const lines = new Set(operation.lines.map((line) => line.id));
  return { ...operation, points: operation.points.filter((point) => used.has(point.id)),
    bindings:operation.bindings?.filter(binding=>binding.target!=="point"||used.has(binding.pointId)),
    constraints: operation.constraints.filter((constraint) =>
      (!constraint.lineId || lines.has(constraint.lineId)) &&
      [constraint.pointId, constraint.firstPointId, constraint.secondPointId].every((id) => !id || used.has(id))) };
}
