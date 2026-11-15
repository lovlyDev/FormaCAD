import { describe,it,expect } from "vitest";
import type { Project } from "../../types";
import { captureSplit } from "./splitCommand";
function project():Project {
 return {id:"project",currentRevision:"rev",revisions:[{id:"rev",program:JSON.stringify({schemaVersion:2,revisionId:"rev",parameters:[],features:[{id:"pad",name:"Pad",operation:{type:"sphere",radius:{kind:"literal",mm:10}}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]})}]} as Project;
}
describe("captured plane split",()=>{
 it("preserves the original document and stages two explicit sides in one batch",()=>{
  const base=project(),before=JSON.stringify(base),edit=captureSplit(base,"body","x",5)!;
  const draft=JSON.parse(edit.previewSource);
  expect(JSON.stringify(base)).toBe(before);
  expect(draft.bodies).toHaveLength(2);
  expect(draft.features.slice(1).map((item:{operation:unknown})=>item.operation)).toEqual([
   {type:"splitPlane",bodyFeatureId:"pad",planeOriginMm:[5,0,0],planeNormal:[1,0,0],side:"positive"},
   {type:"splitPlane",bodyFeatureId:"pad",planeOriginMm:[5,0,0],planeNormal:[1,0,0],side:"negative"}
  ]);
  expect(edit.commands.map(item=>item.command)).toEqual(["add_feature","add_feature","set_body_source","add_body"]);
  expect(edit.captured.base.currentRevision).toBe("rev");
  expect(draft.bodies[0].id).toBe("body");
 });
 it("rejects stale baselines, missing targets and invalid offsets instead of choosing another body",()=>{
  const base=project();
  expect(captureSplit(base,"unknown","x",0)).toBeNull();
  expect(captureSplit(base,"","x",0)).toBeNull();
  expect(captureSplit({...base,currentRevision:"changed"},"body","x",0)).toBeNull();
  for(const value of [NaN,Infinity,10001])expect(captureSplit(base,"body","x",value)).toBeNull();
 });
});
