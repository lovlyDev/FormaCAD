import { describe,expect,it,vi } from "vitest";
import * as THREE from "three";
import { panCamera } from "./panCamera";
for(const projection of ["perspective","orthographic"]){describe(`${projection} relative pan`,()=>{
 it("translates camera and target together while keeping framing and orientation",()=>{
  const camera=projection==="perspective"?new THREE.PerspectiveCamera(38,2,.1,10000):new THREE.OrthographicCamera(-100,100,50,-50,.1,10000);
  camera.position.set(100,80,100);camera.lookAt(0,0,0);camera.zoom=2;camera.updateProjectionMatrix();
  const controls={enabled:true,target:new THREE.Vector3(),update:vi.fn()};const position=camera.position.clone(),rotation=camera.quaternion.clone(),zoom=camera.zoom;
  panCamera(camera,controls,80,-40,800,400);
  expect(camera.position.clone().sub(position).distanceTo(controls.target)).toBeLessThan(1e-9);
  expect(controls.target.length()).toBeGreaterThan(1);expect(camera.quaternion.equals(rotation)).toBe(true);expect(camera.zoom).toBe(zoom);
  expect(controls.update).toHaveBeenCalledOnce();
  const shifted=camera.position.clone();panCamera(camera,controls,NaN,0,800,400);panCamera(camera,controls,1,1,800,0);expect(camera.position.equals(shifted)).toBe(true);
 });
});}
