import {describe,it,expect} from "vitest";
import {previewError} from "./previewError";
describe("native preview diagnostics",()=>{
 it("explains an empty intersection while retaining technical details",()=>{
  const detail='Native CAD: {"code":"EMPTY_INTERSECTION","targetId":"common","message":"Intersection is empty"}';
  expect(previewError(detail)).toEqual({message:"Intersection is empty",targetId:"common",detail});
 });
 it("does not mislabel an arbitrary invalid solid as an empty intersection",()=>{
  expect(previewError('{"code":"INVALID_VOLUME"}').message).toBe("CAD document is invalid");
 });
 it("preserves the generic fallback for non-JSON failures",()=>{
  expect(previewError("Worker unavailable")).toEqual({message:"Model preview failed",targetId:null,detail:"Worker unavailable"});
 });
});
