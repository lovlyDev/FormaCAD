import { invoke } from "@tauri-apps/api/core";
import type { Project } from "../../types";
import { t } from "../../i18n";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import { captureApply } from "../model-apply/capturedApply";
export type SplitAxis = "x" | "y" | "z";
export function captureSplit(project:Project,bodyId:string,axis:SplitAxis,offset:number){
 const revision=project.revisions.find(item=>item.id===project.currentRevision);
 const document=readTypedCadDocument(revision?.program??"");
 const body=document?.bodies.find(item=>item.id===bodyId);
 if(!document||document.revisionId!==project.currentRevision||!body||!Number.isFinite(offset)||Math.abs(offset)>10000)return null;
 const index={x:0,y:1,z:2}[axis],origin=[0,0,0],normal=[0,0,0];origin[index]=offset;normal[index]=1;
 const feature=(side:"positive"|"negative")=>({id:crypto.randomUUID(),name:t(side==="positive"?"Positive split half":"Negative split half"),operation:{type:"splitPlane",bodyFeatureId:body.sourceFeatureId,planeOriginMm:origin,planeNormal:normal,side}});
 const positive=feature("positive"),negative=feature("negative");
 const extra={id:crypto.randomUUID(),name:Array.from(body.name).slice(0,72).join("")+" (-)",sourceFeatureId:negative.id};
 const bodies=[...document.bodies.map(item=>item.id===body.id?{...item,sourceFeatureId:positive.id}:item),extra];
 const commands=[{command:"add_feature",feature:positive},{command:"add_feature",feature:negative},{command:"set_body_source",bodyId:body.id,sourceFeatureId:positive.id},{command:"add_body",body:extra}];
 const captured=captureApply(project,"",t("Split body by plane"));
 return {captured,commands,previewSource:JSON.stringify({...document,features:[...document.features,positive,negative],bodies}),
 invoke:()=>invoke<Project>("apply_ir_commands",{projectId:captured.base.id,expectedRevision:captured.base.currentRevision,prompt:captured.prompt,commands})};
}
