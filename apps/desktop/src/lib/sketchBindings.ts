import type { SketchOperation } from "./sketchDocument";
import type { CadParameter } from "./cadParameters";
export type SketchBinding=NonNullable<SketchOperation["bindings"]>[number];
export function resolveSketchBindings(operation:SketchOperation,parameters:CadParameter[]):SketchOperation {
 const points=operation.points.map(point=>({...point})),originMm=[...operation.originMm] as SketchOperation["originMm"];
 for(const binding of operation.bindings??[]){
  const parameter=parameters.find(item=>item.id===binding.value.parameterId);
  if(!parameter)throw new Error("BROKEN_REFERENCE");
  const coordinate=parameter.valueMm*binding.value.scale+binding.value.offsetMm;
  if(!Number.isFinite(coordinate)||Math.abs(coordinate)>10000)throw new Error("INVALID_VALUE");
  if(binding.target==="origin")originMm[({x:0,y:1,z:2})[binding.axis]]=coordinate;
  else {const point=points.find(item=>item.id===binding.pointId);if(!point)throw new Error("BROKEN_REFERENCE");point[binding.axis==="x"?"xMm":"yMm"]=coordinate;}
 }
 return {...operation,points,originMm};
}
export function unlinkSketchBinding(operation:SketchOperation,parameters:CadParameter[],id:string):SketchOperation {
 // Materialize current effective coordinates so removing a link never moves the profile.
 const resolved=resolveSketchBindings(operation,parameters);
 return {...resolved,bindings:operation.bindings?.filter(item=>item.id!==id)};
}
export function pointIsBound(operation:SketchOperation,id:string,axis?:"x"|"y"){return operation.bindings?.some(binding=>binding.target==="point"&&binding.pointId===id&&(!axis||binding.axis===axis))??false;}
