import { Box3,Object3D } from "three";
export const MIN_GRID_DISTANCE=550;
/** Keep the world origin and translated model inside a readable part of the fade. */
export function gridDistance(object:Object3D):number{
 object.updateWorldMatrix(true,true);const bounds=new Box3().setFromObject(object);
 if(bounds.isEmpty()||![...bounds.min.toArray(),...bounds.max.toArray()].every(Number.isFinite))return MIN_GRID_DISTANCE;
 const x=Math.max(Math.abs(bounds.min.x),Math.abs(bounds.max.x)),y=Math.max(Math.abs(bounds.min.y),Math.abs(bounds.max.y)),z=Math.max(Math.abs(bounds.min.z),Math.abs(bounds.max.z));
 return Math.max(MIN_GRID_DISTANCE,Math.hypot(x,y,z)*2.5);
}
export class GridExtentTransition{
 private elapsed=0;
 constructor(readonly from:number,readonly to:number,readonly duration=.5){}
 step(delta:number){this.elapsed=Math.min(this.duration,this.elapsed+Math.max(0,Math.min(delta,.05)));const t=this.duration>0?this.elapsed/this.duration:1;return this.from+(this.to-this.from)*(t*t*(3-2*t));}
 get done(){return this.elapsed>=this.duration;}
}
