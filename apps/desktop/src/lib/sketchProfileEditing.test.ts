import { describe,expect,it } from "vitest";
import { createStarterSketch,readSketchOperation } from "./sketchDocument";
import { addRectangleContour,removeProfileContour,removeProfilePoint,splitProfileLine } from "./sketchProfileEditing";
import { sketchLoops } from "./sketchTopology";
import { addConstructionLine } from "./sketchConstruction";
import { fixSketchPoint,coincideSketchPoints } from "./sketchPointConstraints";
const starter=()=>readSketchOperation(JSON.parse(createStarterSketch()).features[0].operation)!;
describe("profile editing",()=>{
 it("adds a bounded loop with stable original IDs and removes it without changing the original",()=>{
  const original=starter(),next=addRectangleContour(original,-4,-3,8,6)!;
  expect(next.points).toHaveLength(8);expect(next.constraints).toHaveLength(4);expect(sketchLoops(next)).toHaveLength(2);
  expect(readSketchOperation(JSON.parse(JSON.stringify(next)))).toEqual(next);
  expect(removeProfileContour(next,sketchLoops(next)![1].id)).toEqual(original);
  expect(removeProfileContour(original,sketchLoops(original)![0].id)).toBeNull();
  expect(addRectangleContour(original,9999,0,4,5)).toBeNull();expect(addRectangleContour(original,0,0,0,5)).toBeNull();
  let full=original;while(full.points.length<32)full=addRectangleContour(full,0,0,1,1)!;
  expect(addRectangleContour(full,0,0,1,1)).toBeNull();
 });
 it("splits an edge and clears its dimension while preserving unrelated constraints",()=>{
  const original=starter(),changed=splitProfileLine(original,"line_ab")!;
  expect(original.points).toHaveLength(4);expect(changed.points).toHaveLength(5);expect(sketchLoops(changed)![0].pointIds).toHaveLength(5);
  expect(changed.constraints.some(c=>c.lineId==="line_ab")).toBe(false);
  expect(changed.constraints.filter(c=>c.lineId!=="line_ab")).toEqual(original.constraints.filter(c=>c.lineId!=="line_ab"));
  const guided=addConstructionLine(original,"point_a","point_c")!;
  expect(splitProfileLine(guided,guided.lines.at(-1)!.id)).toBeNull();
 });
 it("removes a vertex, attached guides and point constraints while retaining a triangle",()=>{
  let original=addConstructionLine(starter(),"point_b","point_d")!;
  original=fixSketchPoint(original,"point_b")!;original=coincideSketchPoints(original,"point_b","point_d")!;
  const changed=removeProfilePoint(original,"point_b")!;
  expect(changed.points).toHaveLength(3);expect(changed.lines).toHaveLength(3);expect(sketchLoops(changed)![0].pointIds).toHaveLength(3);
  expect(changed.constraints.some(c=>c.pointId==="point_b"||c.firstPointId==="point_b"||c.secondPointId==="point_b")).toBe(false);
  expect(removeProfilePoint(changed,"point_a")).toBeNull();
 });
 it("rejects duplicate fixed or coincidence constraints in either order",()=>{
  const original=starter(),fixed=fixSketchPoint(original,"point_a")!;
  expect(fixed.constraints.at(-1)).toMatchObject({pointId:"point_a",xMm:original.points[0].xMm,yMm:original.points[0].yMm});
  expect(fixSketchPoint(fixed,"point_a")).toBeNull();expect(fixSketchPoint(original,"missing")).toBeNull();
  const next=coincideSketchPoints(original,"point_a","point_c")!;
  expect(coincideSketchPoints(next,"point_c","point_a")).toBeNull();expect(coincideSketchPoints(original,"point_a","point_a")).toBeNull();
 });
});
