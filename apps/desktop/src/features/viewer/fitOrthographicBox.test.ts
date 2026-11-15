import { expect, it } from "vitest";
import * as THREE from "three";
import { fitOrthographicBox } from "./fitOrthographicBox";

it("fits off-origin bounds in wide and narrow orthographic views without deferred state", () => {
  for (const aspect of [0.5, 2]) {
    const camera=new THREE.OrthographicCamera(-100*aspect,100*aspect,100,-100,0.1,10000);
    camera.position.set(200,150,180);camera.lookAt(0,0,0);camera.updateMatrixWorld(true);
    const controls={enabled:true,target:new THREE.Vector3(),update:()=>{camera.lookAt(controls.target);camera.updateMatrixWorld(true);}};
    const box=new THREE.Box3(new THREE.Vector3(100,-20,30),new THREE.Vector3(180,40,70));
    const orientation=camera.quaternion.clone();
    expect(fitOrthographicBox(camera,controls,box)).toBe(true);
    expect(controls.target.toArray()).toEqual([140,10,50]);
    expect(camera.quaternion.angleTo(orientation)).toBeLessThan(1e-7);
    for(const x of [100,180])for(const y of [-20,40])for(const z of [30,70]){
      const p=new THREE.Vector3(x,y,z).project(camera);
      expect(Math.abs(p.x)).toBeLessThanOrEqual(1/1.8+1e-12);expect(Math.abs(p.y)).toBeLessThanOrEqual(1/1.8+1e-12);
    }
    const before=camera.toJSON();expect(fitOrthographicBox(camera,controls,new THREE.Box3())).toBe(false);expect(camera.toJSON()).toEqual(before);
  }
});
