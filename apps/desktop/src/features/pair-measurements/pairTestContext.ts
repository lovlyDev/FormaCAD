import { newProject } from "../../stores/workspace";
import { defaults } from "../../types";
import type { FaceSelection } from "../viewer/faceSelection";
import type { PairCapture } from "./pairCapture";
import type { PairReport } from "./pairSchema";
export function pairTestContext(){
 const project=newProject("Pair","blank","mm","codex"),head="c661482b-378a-4fa3-a426-ad9bf6621478";project.currentRevision=head;
 project.revisions=[{id:head,parent:null,createdAt:project.createdAt,prompt:"Source",parameters:defaults,source:"source.step",program:JSON.stringify({schemaVersion:2,revisionId:head,parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle"}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile"}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]})}];
 project.files=[{name:"source.step",kind:"model",sha256:"a".repeat(64),size:100}];
 const face:FaceSelection={bodyId:"body",revisionId:head,sceneToken:"real-object-uuid-a",faceOrdinal:99,topologyRef:{schemaVersion:1,kind:"face",ownerFeatureId:"pad",role:"box-face:zmax",occurrencePath:[]}};
 const second:FaceSelection={...face,faceOrdinal:7,topologyRef:{...face.topologyRef!,role:"box-face:zmin"}};
 return{project,bodyId:"body",sceneToken:"real-object-uuid-a",interactive:true,edge:null,face,second};
}
export function pairTestReport(capture:PairCapture):PairReport{
 return{projectId:capture.request.projectId,schemaVersion:1,requestId:"0fdca3bb-60ac-4014-99e9-28b8306687c7",engine:{name:"occt",protocolVersion:1},evaluationSource:"rebuiltAuthoredBody",revisionId:capture.request.expectedRevision,bodyId:capture.request.bodyId,documentSha256:"a".repeat(64),sourceSha256:capture.sourceSha256,sourceSize:capture.sourceSize,importedAssetSeals:[],query:capture.request.query,result:{kind:"minimumDistance",distanceMm:10,pointAMm:[0,0,0],pointBMm:[0,0,10]}};
}
