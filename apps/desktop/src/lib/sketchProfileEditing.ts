import type { SketchOperation } from "./sketchDocument";
import { retainUsedGeometry, sketchId } from "./sketchGeometry";
import { sketchLoops } from "./sketchTopology";

export function addRectangleContour(operation: SketchOperation, x: number, y: number, width: number, height: number): SketchOperation | null {
  const loops = sketchLoops(operation);
  if (!loops || loops.length >= 8 || operation.points.length + 4 > 32 || operation.lines.length + 4 > 64
    || operation.constraints.length + (operation.bindings?.length??0) + 4 > 64 || ![x,y,width,height,x+width,y+height].every((value) => Number.isFinite(value) && Math.abs(value) <= 10000)
    || width <= 0 || height <= 0) return null;
  let next = operation;
  const pointIds: string[] = [];
  for (const [xMm,yMm] of [[x,y],[x+width,y],[x+width,y+height],[x,y+height]]) {
    const id = sketchId(next, "profile_point"); pointIds.push(id);
    next = { ...next, points: [...next.points, { id,xMm,yMm }] };
  }
  for (let index=0;index<4;index++) {
    const id = sketchId(next,"profile_line");
    next = { ...next, lines:[...next.lines,{id,startPointId:pointIds[index],endPointId:pointIds[(index+1)%4]}] };
    const constraint = { id:sketchId(next,"profile_constraint"), kind:index%2===0?"horizontal" as const:"vertical" as const, lineId:id };
    next = { ...next, constraints:[...next.constraints,constraint] };
  }
  return next;
}

export function splitProfileLine(operation: SketchOperation, lineId: string): SketchOperation | null {
  const line = operation.lines.find((item) => item.id===lineId && !item.construction);
  if (!line || operation.points.length>=32 || operation.lines.length>=64) return null;
  const start=operation.points.find((p)=>p.id===line.startPointId),end=operation.points.find((p)=>p.id===line.endPointId);
  if (!start || !end) return null;
  const pointId=sketchId(operation,"profile_point");
  const next={...operation,points:[...operation.points,{id:pointId,xMm:(start.xMm+end.xMm)/2,yMm:(start.yMm+end.yMm)/2}]};
  const second={id:sketchId(next,"profile_line"),startPointId:pointId,endPointId:line.endPointId};
  return {...next,lines:operation.lines.flatMap((item)=>item.id===lineId?[{...item,endPointId:pointId},second]:[item]),
    constraints:operation.constraints.filter((constraint)=>constraint.lineId!==lineId)};
}

export function removeProfilePoint(operation: SketchOperation, pointId: string): SketchOperation | null {
  const loops=sketchLoops(operation),loop=loops?.find((item)=>item.pointIds.includes(pointId));
  if (!loop || loop.pointIds.length<=3) return null;
  const incoming=operation.lines.find((line)=>!line.construction && line.endPointId===pointId)!;
  const outgoing=operation.lines.find((line)=>!line.construction && line.startPointId===pointId)!;
  const affected = new Set(operation.lines.filter((line)=>line.startPointId===pointId||line.endPointId===pointId).map((line)=>line.id));
  const lines=operation.lines.flatMap((line)=>line.id===incoming.id?[{...line,endPointId:outgoing.endPointId}]:affected.has(line.id)?[]:[line]);
  return retainUsedGeometry({...operation,lines,constraints:operation.constraints.filter((c)=>!c.lineId||!affected.has(c.lineId))});
}

export function removeProfileContour(operation: SketchOperation, loopId: string): SketchOperation | null {
  const loops=sketchLoops(operation),loop=loops?.find((item)=>item.id===loopId);
  if (!loops || loops.length<2 || !loop) return null;
  const points=new Set(loop.pointIds);
  return retainUsedGeometry({...operation,lines:operation.lines.filter((line)=>!points.has(line.startPointId)&&!points.has(line.endPointId))});
}
