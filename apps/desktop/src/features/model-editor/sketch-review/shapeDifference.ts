import type { SketchOperation } from "../../../lib/sketchDocument";
export function shapeDifference(points:SketchOperation["points"],solved:SketchOperation["points"]){const byId=new Map(solved.map(point=>[point.id,point]));return points.length!==solved.length||points.some(point=>{const next=byId.get(point.id);return !next||Math.hypot(point.xMm-next.xMm,point.yMm-next.yMm)>1e-5;});}
