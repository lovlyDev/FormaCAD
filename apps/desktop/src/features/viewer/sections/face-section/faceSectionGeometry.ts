import type { SectionPlane,VectorMm } from "../sectionPlane";
interface Geometry {plane:SectionPlane;totalLengthMm:number;curves:{id:string;lengthMm:number;closed:boolean;pointsMm:VectorMm[]}[]}
/** Mirrors the native report geometry invariants after bounded schema parsing. */
export function validFaceSectionGeometry(geometry:Geometry){
 let total=0;
 for(const [index,curve] of geometry.curves.entries()){
  if(curve.id!==`section-edge-${index+1}`)return false;
  for(const point of curve.pointsMm){const signed=point.reduce((sum,value,axis)=>sum+(value-geometry.plane.originMm[axis])*geometry.plane.normal[axis],0);if(!Number.isFinite(signed)||Math.abs(signed)>1e-6)return false;}
  const first=curve.pointsMm[0],last=curve.pointsMm.at(-1)!;
  const closed=first.reduce((sum,value,axis)=>sum+(value-last[axis])**2,0)<=1e-12;
  if(closed!==curve.closed)return false;total+=curve.lengthMm;
 }
 return Math.abs(total-geometry.totalLengthMm)<=1e-9*Math.max(1,Math.abs(total),Math.abs(geometry.totalLengthMm));
}
