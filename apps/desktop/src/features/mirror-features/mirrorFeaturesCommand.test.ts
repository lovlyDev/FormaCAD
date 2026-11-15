import {describe,it,expect} from "vitest";
import type {Project} from "../../types";
import {captureMirrorFeatures,mirrorRange} from "./mirrorFeaturesCommand";
function fixture():Project{return {id:"project",currentRevision:"rev",revisions:[{id:"rev",program:JSON.stringify({schemaVersion:2,revisionId:"rev",parameters:[],features:[
 {id:"seed",name:"Seed",operation:{type:"sphere",radius:{kind:"literal",mm:10}}},
 {id:"round",name:"Round",operation:{type:"fillet",bodyFeatureId:"seed",radius:{kind:"literal",mm:1}}},
 {id:"bevel",name:"Bevel",operation:{type:"chamfer",bodyFeatureId:"round",distance:{kind:"literal",mm:1}}}
],bodies:[{id:"body",name:"Body",sourceFeatureId:"bevel"}]})}]} as Project;}
describe("captured feature mirror",()=>{
 it("keeps original features and records the ordered range and its material base",()=>{
  const project=fixture(),before=JSON.stringify(project),edit=captureMirrorFeatures(project,"body","round","x",0)!;
  const doc=JSON.parse(edit.previewSource);
  expect(JSON.stringify(project)).toBe(before);
  expect(doc.features.at(-1).operation).toEqual({type:"mirrorFeatures",bodyFeatureId:"bevel",baseFeatureId:"seed",featureIds:["round","bevel"],planeOriginMm:[0,0,0],planeNormal:[1,0,0]});
  expect(edit.commands.map(c=>c.command)).toEqual(["add_feature","set_body_source"]);
  expect(captureMirrorFeatures(project,"body","seed","x",5)!.commands[0].feature?.operation.baseFeatureId).toBeNull();
 });
 it("refuses stale, missing or ineligible ranges without silently choosing a feature",()=>{
  const project=fixture();
  expect(captureMirrorFeatures(project,"body","","x",0)).toBeNull();
  expect(captureMirrorFeatures(project,"missing","round","x",0)).toBeNull();
  expect(captureMirrorFeatures({...project,currentRevision:"changed"},"body","round","x",0)).toBeNull();
  for(const offset of [NaN,Infinity,10001])expect(captureMirrorFeatures(project,"body","round","x",offset)).toBeNull();
  const source=JSON.parse(project.revisions[0].program!);source.features.push({id:"moved",name:"Move",operation:{type:"translate",bodyFeatureId:"bevel",offsetMm:[1,0,0]}});source.bodies[0].sourceFeatureId="moved";
  project.revisions[0].program=JSON.stringify(source);expect(mirrorRange(project,"body")).toEqual([]);
 });
});
