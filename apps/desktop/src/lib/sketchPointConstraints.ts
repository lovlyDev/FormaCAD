import type { SketchOperation } from "./sketchDocument";
import { sketchId } from "./sketchGeometry";
export function fixSketchPoint(operation: SketchOperation, pointId: string): SketchOperation | null {
  const point=operation.points.find((p)=>p.id===pointId);
  if (!point || operation.constraints.length+(operation.bindings?.length??0)>=64 || operation.constraints.some((c)=>c.kind==="fixed"&&c.pointId===pointId)) return null;
  return {...operation,constraints:[...operation.constraints,{id:sketchId(operation,"fixed_point"),kind:"fixed",pointId,xMm:point.xMm,yMm:point.yMm}]};
}
export function coincideSketchPoints(operation: SketchOperation, firstPointId: string, secondPointId: string): SketchOperation | null {
  if (firstPointId===secondPointId || operation.constraints.length+(operation.bindings?.length??0)>=64 || ![firstPointId,secondPointId].every((id)=>operation.points.some((p)=>p.id===id))
    || operation.constraints.some((c)=>c.kind==="coincident"&&((c.firstPointId===firstPointId&&c.secondPointId===secondPointId)||(c.firstPointId===secondPointId&&c.secondPointId===firstPointId)))) return null;
  return {...operation,constraints:[...operation.constraints,{id:sketchId(operation,"coincident_points"),kind:"coincident",firstPointId,secondPointId}]};
}
