import { expect,it } from "vitest";
import { SketchDragSession } from "./dragSession";
import { createStarterSketch } from "../../../lib/sketchDocument";
it("keeps the grab offset and immutable screen basis through repeated moves",()=>{
 const basis={a:.5,b:0,c:0,d:.5},point={id:"point_a",xMm:-20,yMm:-10};const drag=new SketchDragSession(7,point,100,100,basis);basis.a=100;
 expect(drag.move(7,100,100)).toEqual(point);
 expect(drag.move(7,120,130)).toEqual({...point,xMm:-10,yMm:-25});
 expect(drag.move(8,900,900)).toEqual({...point,xMm:-10,yMm:-25});
 expect(drag.move(7,110,110)).toEqual({...point,xMm:-15,yMm:-15});
 expect(point.xMm).toBe(-20);
 const op=JSON.parse(createStarterSketch()).features[0].operation;const changed=drag.apply(op);
 expect(changed.points[0].xMm).toBe(-15);expect(op.points[0].xMm).toBe(-20);expect(changed.lines).toBe(op.lines);
 expect(drag.move(7,Infinity,0)).toEqual(changed.points[0]);
 expect(drag.move(7,1000000,0)).toEqual(changed.points[0]);
});
