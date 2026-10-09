import type { SketchOperation } from "../../../lib/sketchDocument";
export type SketchPoint=SketchOperation["points"][number];
export type InverseTransform={a:number;b:number;c:number;d:number};
/** Snapshot the gesture basis once. Changes elsewhere in the modal cannot move the grabbed point. */
export class SketchDragSession {
 readonly pointerId:number;
 private readonly start:SketchPoint;
 private readonly screen:{x:number;y:number};
 private readonly transform:InverseTransform;
 current:SketchPoint;
 constructor(pointerId:number,point:SketchPoint,x:number,y:number,inverse:InverseTransform){
  this.pointerId=pointerId;this.start={...point};this.current={...point};this.screen={x,y};this.transform={a:inverse.a,b:inverse.b,c:inverse.c,d:inverse.d};
 }
 move(pointerId:number,x:number,y:number){
  if(pointerId!==this.pointerId||!Number.isFinite(x)||!Number.isFinite(y))return this.current;
  const dx=x-this.screen.x,dy=y-this.screen.y;
  const xMm=this.start.xMm+this.transform.a*dx+this.transform.c*dy;
  const yMm=this.start.yMm-this.transform.b*dx-this.transform.d*dy;
  if(!Number.isFinite(xMm)||!Number.isFinite(yMm)||Math.abs(xMm)>10000||Math.abs(yMm)>10000)return this.current;
  this.current={...this.start,xMm:Math.round(xMm*100)/100,yMm:Math.round(yMm*100)/100};return this.current;
 }
 get changed(){return this.current.xMm!==this.start.xMm||this.current.yMm!==this.start.yMm;}
 apply(operation:SketchOperation){return {...operation,points:operation.points.map(point=>point.id===this.current.id?{...point,xMm:this.current.xMm,yMm:this.current.yMm}:point)};}
}
