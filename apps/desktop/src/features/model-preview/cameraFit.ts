import * as THREE from "three";
export type CameraPose={position:THREE.Vector3;target:THREE.Vector3};
export function fitPose(camera:THREE.PerspectiveCamera,target:THREE.Vector3,object:THREE.Object3D):CameraPose|null{
 object.updateWorldMatrix(true,true);const bounds=new THREE.Box3().setFromObject(object);
 if(bounds.isEmpty()||![...bounds.min.toArray(),...bounds.max.toArray()].every(Number.isFinite))return null;
 const sphere=bounds.getBoundingSphere(new THREE.Sphere()),radius=Math.max(sphere.radius,.01);
 const halfVertical=THREE.MathUtils.degToRad(camera.fov/2),halfHorizontal=Math.atan(Math.tan(halfVertical)*camera.aspect);
 const distance=radius/Math.sin(Math.min(halfVertical,halfHorizontal))*1.15;
 const direction=camera.position.clone().sub(target);if(direction.lengthSq()<1e-10)direction.set(1,.8,1);direction.normalize();
 return {position:sphere.center.clone().addScaledVector(direction,distance),target:sphere.center.clone()};
}
export class CameraFitTransition{
 private elapsed=0;
 constructor(readonly from:CameraPose,readonly to:CameraPose,readonly duration=.38){}
 step(delta:number):CameraPose{
  this.elapsed=Math.min(this.duration,this.elapsed+Math.max(0,Math.min(delta,.05)));
  const t=this.duration>0?this.elapsed/this.duration:1,eased=t*t*(3-2*t);
  return {position:this.from.position.clone().lerp(this.to.position,eased),target:this.from.target.clone().lerp(this.to.target,eased)};
 }
 get done(){return this.elapsed>=this.duration;}
}
