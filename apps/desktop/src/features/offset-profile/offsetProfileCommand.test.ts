import {it,expect} from "vitest";
import type {Project} from "../../types";
import {captureOffsetProfile,offsetProfileBodies} from "./offsetProfileCommand";
function fixture():Project{return {id:"project",currentRevision:"rev",revisions:[{id:"rev",program:JSON.stringify({schemaVersion:2,revisionId:"rev",parameters:[{id:"height",name:"Height",valueMm:10}],features:[
 {id:"profile",name:"Profile",operation:{type:"rectangle",width:{kind:"literal",mm:40},depth:{kind:"literal",mm:20}}},
 {id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:{kind:"parameter",parameterId:"height"}}}
],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]})}]} as Project;}
it("appends offset and extrusion atomically while preserving the source history and distance parameter",()=>{
 const project=fixture(),before=JSON.stringify(project),edit=captureOffsetProfile(project,"body",-2,"rounded")!;
 expect(JSON.stringify(project)).toBe(before);const doc=JSON.parse(edit.previewSource);
 expect(doc.features.slice(0,2)).toEqual(JSON.parse(project.revisions[0].program!).features);
 expect(doc.features[2].operation).toEqual({type:"offsetProfile",sketchId:"profile",offset:{kind:"literal",mm:-2},join:"rounded"});
 expect(doc.features[3].operation.distance).toEqual({kind:"parameter",parameterId:"height"});
 expect(doc.features[3].operation.sketchId).toBe(doc.features[2].id);
 expect(doc.bodies[0].sourceFeatureId).toBe(doc.features[3].id);
 expect(edit.commands.map(c=>c.command)).toEqual(["add_feature","add_feature","set_body_source"]);
});
it("requires an eligible explicit target and rejects invalid offsets and stale documents",()=>{
 const project=fixture();for(const value of [NaN,Infinity,0,10001,1e-8])expect(captureOffsetProfile(project,"body",value,"intersection")).toBeNull();
 expect(captureOffsetProfile(project,"",2,"intersection")).toBeNull();
 expect(offsetProfileBodies({...project,currentRevision:"changed"})).toEqual([]);
 const doc=JSON.parse(project.revisions[0].program!);doc.features[1].suppressed=true;project.revisions[0].program=JSON.stringify(doc);
 expect(offsetProfileBodies(project)).toEqual([]);
});
