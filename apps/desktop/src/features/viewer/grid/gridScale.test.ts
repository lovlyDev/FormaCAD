import { expect,it } from "vitest";
import { OrthographicCamera,PerspectiveCamera,Vector3 } from "three";
import { gridScale } from "./gridScale";
it("preserves the baseline nearby and coarsens distant perspective views",()=>{
 const camera=new PerspectiveCamera(38,1,.1,100000);camera.position.set(0,100,100);camera.lookAt(0,0,0);expect(gridScale(camera,900,new Vector3())).toEqual({cell:10,blend:0});
 camera.position.multiplyScalar(100);camera.lookAt(0,0,0);const far=gridScale(camera,900,new Vector3());expect(far.cell).toBeGreaterThanOrEqual(100);expect(far.blend).toBeGreaterThanOrEqual(0);expect(far.blend).toBeLessThanOrEqual(1);
});
it("uses orthographic zoom and viewport size rather than camera distance",()=>{
 const camera=new OrthographicCamera(-5000,5000,5000,-5000,.1,100000);camera.position.z=100;const distant=gridScale(camera,800,new Vector3());camera.position.z=50000;expect(gridScale(camera,800,new Vector3())).toEqual(distant);camera.zoom=100;expect(gridScale(camera,800,new Vector3()).cell).toBe(10);expect(gridScale(camera,1,new Vector3()).cell).toBeGreaterThan(10);
});
