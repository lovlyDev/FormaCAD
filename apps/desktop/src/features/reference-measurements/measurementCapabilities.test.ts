import { expect,it } from "vitest";
import { newProject } from "../../stores/workspace";
import { defaults } from "../../types";
import { supportedMeasurementQueries } from "./measurementCapabilities";
import type { EdgeSelection } from "../viewer/edgeSelection";
import type { FaceSelection } from "../viewer/faceSelection";

const head="c661482b-378a-4fa3-a426-ad9bf6621478";
function project(sketch="circle"){
 const value=newProject("Part","blank","mm","codex");value.currentRevision=head;
 const program=JSON.stringify({schemaVersion:2,revisionId:head,parameters:[],features:[{id:"profile",name:"Profile",operation:{type:sketch}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile"}},{id:"turned",name:"Turned",operation:{type:"rotate",bodyFeatureId:"pad"}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"turned"}]});
 value.revisions=[{id:head,parent:null,createdAt:value.createdAt,prompt:"Source",parameters:defaults,program}];return value;
}
const edge:EdgeSelection={bodyId:"body",edgeOrdinal:99,revisionId:head,topologyRef:{schemaVersion:1,kind:"edge",ownerFeatureId:"pad",role:"cylinder-edge:top",occurrencePath:["turned"]}};
const face:FaceSelection={bodyId:"body",faceOrdinal:99,revisionId:head,topologyRef:{schemaVersion:1,kind:"face",ownerFeatureId:"pad",role:"cylinder-face:side",occurrencePath:["turned"]}};
const kinds=(...args:Parameters<typeof supportedMeasurementQueries>)=>supportedMeasurementQueries(...args).map(query=>query.kind);
it("offers native length, radius and diameter only for an owned circular edge, never from mesh ordinals",()=>{
 expect(kinds(project(),"body",edge,null)).toEqual(["bodyMetrics","edgeLength","edgeRadius","edgeDiameter"]);
 expect(kinds(project(),"body",{...edge,topologyRef:undefined},null)).toEqual(["bodyMetrics"]);
 expect(kinds(project("rectangle"),"body",{...edge,topologyRef:{...edge.topologyRef!,role:"box-edge:x:ymin:zmin"}},null)).toEqual(["bodyMetrics","edgeLength"]);
 expect(kinds(project("rectangle"),"body",edge,null)).toEqual(["bodyMetrics"]);
});
it("offers plane data for authored caps but only area for the cylinder side",()=>{
 expect(kinds(project(),"body",null,face)).toEqual(["bodyMetrics","faceArea"]);
 expect(kinds(project(),"body",null,{...face,topologyRef:{...face.topologyRef!,role:"cylinder-face:bottom"}})).toEqual(["bodyMetrics","faceArea","planarFace"]);
});
it("rejects stale body, head and occurrence routes without substituting geometric lookalikes",()=>{
 expect(kinds(project(),"body",{...edge,revisionId:"older"},null)).toEqual(["bodyMetrics"]);
 expect(kinds(project(),"body",{...edge,bodyId:"copy"},null)).toEqual(["bodyMetrics"]);
 expect(kinds(project(),"body",{...edge,topologyRef:{...edge.topologyRef!,occurrencePath:["another"]}},null)).toEqual(["bodyMetrics"]);
 expect(kinds(project(),"missing",edge,null)).toEqual([]);
});
