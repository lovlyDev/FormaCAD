import { expect,it } from "vitest";
import { Vector3,PerspectiveCamera,Mesh,BoxGeometry } from "three";
import { CameraFitTransition,fitPose } from "./cameraFit";
it("fits the actual bounds and preserves viewing direction",()=>{
 const camera=new PerspectiveCamera(38,.4);camera.position.set(100,80,100);const object=new Mesh(new BoxGeometry(40,20,10));object.position.set(50,10,-8);
 const pose=fitPose(camera,new Vector3(),object)!;expect(pose.target.toArray()).toEqual([50,10,-8]);expect(pose.position.clone().sub(pose.target).normalize().distanceTo(camera.position.clone().normalize())).toBeLessThan(1e-12);
 expect(pose.position.distanceTo(pose.target)).toBeGreaterThan(100);
});
it("moves monotonically without overshoot and retargets from the current pose",()=>{
 const from={position:new Vector3(100,80,100),target:new Vector3()},to={position:new Vector3(20,20,20),target:new Vector3(5,0,0)};
 const motion=new CameraFitTransition(from,to);let distance=Infinity,pose=from;
 for(let n=0;n<30;n++){pose=motion.step(1/60);const next=pose.position.distanceTo(to.position);expect(next).toBeLessThanOrEqual(distance);distance=next;}
 expect(motion.done).toBe(true);expect(pose.position.toArray()).toEqual(to.position.toArray());
 const retarget=new CameraFitTransition(pose,from);expect(retarget.step(0).position.toArray()).toEqual(pose.position.toArray());
 expect(new CameraFitTransition(from,to,0).step(.01).position.toArray()).toEqual(to.position.toArray());
});
