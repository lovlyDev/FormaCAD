import { OrthographicCamera,PerspectiveCamera,Vector3,type Camera } from "three";
export function gridScale(camera:Camera,height:number,center:Vector3){
 let worldPerPixel=1;
 if(camera instanceof OrthographicCamera)worldPerPixel=(camera.top-camera.bottom)/camera.zoom/Math.max(1,height);
 else if(camera instanceof PerspectiveCamera){const depth=Math.max(.01,center.clone().sub(camera.position).dot(camera.getWorldDirection(new Vector3())));worldPerPixel=2*depth*Math.tan(camera.getEffectiveFOV()*Math.PI/360)/Math.max(1,height);}
 const level=Math.max(0,Math.log10(Math.max(1,worldPerPixel*20/10))),base=Math.floor(level),blend=level-base;
 return {cell:10*10**base,blend:blend*blend*(3-2*blend)};
}
