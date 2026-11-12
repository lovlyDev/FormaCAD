import { describe,it,expect } from "vitest";
import type { Project } from "../../types";
import type { FaceSelection } from "../viewer/faceSelection";
import { captureThicken,thickenIntent } from "./thickenCommand";
function fixture(){
 const project={id:"project",currentRevision:"rev",revisions:[{id:"rev",program:JSON.stringify({schemaVersion:2,revisionId:"rev",parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle",width:{kind:"literal",mm:40},depth:{kind:"literal",mm:20}}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:{kind:"literal",mm:10}}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]})}]} as Project;
 const selection:FaceSelection={bodyId:"body",revisionId:"rev",faceOrdinal:1,topologyRef:{schemaVersion:1,kind:"face",ownerFeatureId:"pad",role:"box-face:zmax",occurrencePath:[]}};
 return {project,selection};
}
describe("captured face thickening",()=>{
 it("stages a new body without replacing the original and freezes its exact face reference",()=>{
  const {project,selection}=fixture(),before=JSON.stringify(project);
  const intent=thickenIntent(project,selection)!;
  selection.topologyRef!.role="box-face:zmin";
  const edit=captureThicken(project,intent,2)!;
  const draft=JSON.parse(edit.previewSource);
  expect(JSON.stringify(project)).toBe(before);
  expect(draft.bodies[0].sourceFeatureId).toBe("pad");
  expect(draft.bodies).toHaveLength(2);
  expect(draft.features.at(-1).operation.reference.role).toBe("box-face:zmax");
  expect(edit.commands.map(c=>c.command)).toEqual(["add_feature","add_body"]);
  expect(edit.captured.base.currentRevision).toBe("rev");
 });
 it("refuses stale project/revision/body routes and invalid thickness without retargeting",()=>{
  const {project,selection}=fixture(),intent=thickenIntent(project,selection)!;
  expect(thickenIntent(project,null)).toBeNull();
  expect(thickenIntent(project,{...selection,topologyRef:undefined})).toBeNull();
  expect(thickenIntent(project,{...selection,bodyId:"missing"})).toBeNull();
  expect(captureThicken({...project,id:"another"},intent,2)).toBeNull();
  expect(captureThicken({...project,currentRevision:"changed"},intent,2)).toBeNull();
  for(const offset of [0,NaN,Infinity,10001])expect(captureThicken(project,intent,offset)).toBeNull();
  expect(captureThicken(project,intent,-2)).not.toBeNull();
 });
});
