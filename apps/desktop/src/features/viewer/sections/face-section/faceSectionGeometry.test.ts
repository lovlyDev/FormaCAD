import { expect,it } from "vitest";
import { validFaceSectionGeometry } from "./faceSectionGeometry";
import type { VectorMm } from "../sectionPlane";
const geometry={plane:{originMm:[0,0,8] as VectorMm,normal:[0,0,1] as VectorMm,deflectionMm:.01},totalLengthMm:10,curves:[{id:"section-edge-1",lengthMm:10,closed:false,pointsMm:[[0,0,8],[10,0,8]] as VectorMm[]}]};
it("rejects wrong curve order, off-plane points, false closure and inconsistent exact totals",()=>{
 expect(validFaceSectionGeometry(geometry)).toBe(true);
 expect(validFaceSectionGeometry({...geometry,totalLengthMm:9})).toBe(false);
 expect(validFaceSectionGeometry({...geometry,curves:[{...geometry.curves[0],id:"section-edge-2"}]})).toBe(false);
 expect(validFaceSectionGeometry({...geometry,curves:[{...geometry.curves[0],closed:true}]})).toBe(false);
 expect(validFaceSectionGeometry({...geometry,curves:[{...geometry.curves[0],pointsMm:[[0,0,9],[10,0,8]]}]})).toBe(false);
 expect(validFaceSectionGeometry({...geometry,curves:[],totalLengthMm:0})).toBe(true);
});
