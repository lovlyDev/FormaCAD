import { invoke } from "@tauri-apps/api/core";
import type { Project } from "../../types";
import { t } from "../../i18n";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import { captureApply } from "../model-apply/capturedApply";
import type { FaceSelection } from "../viewer/faceSelection";
import type { FaceTopologyReference } from "../viewer/topology/topologyReference";
import { matchesReferenceRoute } from "../viewer/topology/referenceRoute";

export interface ThickenIntent {
 projectId:string; revisionId:string; bodyId:string; reference:FaceTopologyReference;
}
function target(project:Project,intent:ThickenIntent){
 if(project.id!==intent.projectId||project.currentRevision!==intent.revisionId)return null;
 const document=readTypedCadDocument(project.revisions.find(r=>r.id===project.currentRevision)?.program??"");
 const body=document?.bodies.find(b=>b.id===intent.bodyId);
 if(!document||document.revisionId!==intent.revisionId||!body||!matchesReferenceRoute(document,body.sourceFeatureId,intent.reference))return null;
 return {document,body};
}
export function thickenIntent(project:Project,face:FaceSelection|null):ThickenIntent|null {
 if(!face?.topologyRef||!face.revisionId)return null;
 const intent={projectId:project.id,revisionId:face.revisionId,bodyId:face.bodyId,reference:structuredClone(face.topologyRef)};
 return target(project,intent)?intent:null;
}
export function captureThicken(project:Project,intent:ThickenIntent,offset:number){
 const input=target(project,intent);
 if(!input||!Number.isFinite(offset)||Math.abs(offset)<=1e-7||Math.abs(offset)>10000)return null;
 const {document,body}=input,name=t("Thicken face");
 const feature={id:crypto.randomUUID(),name,operation:{type:"thickenFace",bodyFeatureId:body.sourceFeatureId,reference:structuredClone(intent.reference),offset:{kind:"literal",mm:offset}}};
 const newBody={id:crypto.randomUUID(),name:t("Thickened face"),sourceFeatureId:feature.id};
 const commands=[{command:"add_feature",feature},{command:"add_body",body:newBody}];
 const captured=captureApply(project,"",name);
 return {captured,commands,previewSource:JSON.stringify({...document,features:[...document.features,feature],bodies:[...document.bodies,newBody]}),
  invoke:()=>invoke<Project>("apply_ir_commands",{projectId:captured.base.id,expectedRevision:captured.base.currentRevision,prompt:captured.prompt,commands})};
}
