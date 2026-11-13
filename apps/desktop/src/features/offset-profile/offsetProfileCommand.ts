import {invoke} from "@tauri-apps/api/core";
import type {Project} from "../../types";
import {t} from "../../i18n";
import {readTypedCadDocument} from "../../lib/typedCadDocument";
import {captureApply} from "../model-apply/capturedApply";
export type ProfileJoin="intersection"|"rounded";
function document(project:Project){
 const doc=readTypedCadDocument(project.revisions.find(r=>r.id===project.currentRevision)?.program??"");
 return doc?.revisionId===project.currentRevision?doc:null;
}
export function offsetProfileBodies(project:Project){
 const doc=document(project);if(!doc)return [];
 const features=new Map(doc.features.map(feature=>[feature.id,feature]));
 return doc.bodies.flatMap(body=>{
  const extrusion=features.get(body.sourceFeatureId),op=extrusion?.operation;
  const profile=typeof op?.sketchId==="string"?features.get(op.sketchId):null;
  return op?.type==="extrude"&&!extrusion?.suppressed&&profile&&!profile.suppressed&&["rectangle","circle","sketch2d","offsetProfile"].includes(profile.operation.type)
   ?[{...body,extrusion:extrusion!,profile}]:[];
 });
}
export function captureOffsetProfile(project:Project,bodyId:string,offset:number,join:ProfileJoin){
 const doc=document(project),target=offsetProfileBodies(project).find(body=>body.id===bodyId);
 if(!doc||!target||!Number.isFinite(offset)||Math.abs(offset)<=1e-7||Math.abs(offset)>10000||!["intersection","rounded"].includes(join))return null;
 const feature={id:crypto.randomUUID(),name:t("Offset profile"),operation:{type:"offsetProfile",sketchId:target.profile.id,offset:{kind:"literal",mm:offset},join}};
 const extrusion={id:crypto.randomUUID(),name:t("Offset extrusion"),operation:{...target.extrusion.operation,sketchId:feature.id}};
 const commands=[{command:"add_feature",feature},{command:"add_feature",feature:extrusion},{command:"set_body_source",bodyId,sourceFeatureId:extrusion.id}];
 const captured=captureApply(project,"",t("Offset profile"));
 return {captured,commands,previewSource:JSON.stringify({...doc,features:[...doc.features,feature,extrusion],bodies:doc.bodies.map(body=>body.id===bodyId?{...body,sourceFeatureId:extrusion.id}:body)}),
  invoke:()=>invoke<Project>("apply_ir_commands",{projectId:captured.base.id,expectedRevision:captured.base.currentRevision,prompt:captured.prompt,commands})};
}
