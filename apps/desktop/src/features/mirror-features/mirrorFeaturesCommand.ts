import {invoke} from "@tauri-apps/api/core";
import type {Project} from "../../types";
import {t} from "../../i18n";
import {readTypedCadDocument,type TypedCadFeature} from "../../lib/typedCadDocument";
import {captureApply} from "../model-apply/capturedApply";
export type MirrorAxis="x"|"y"|"z";
const modifiers=new Set(["hole","fillet","filletEdge","filletReferencedEdge","chamfer","chamferReferencedEdge","extrudeFace","shell","splitPlane","patternLinear","patternCircular","mirrorFeatures"]);
const generators=new Set(["extrude","revolve","sphere","cone","torus","importStep","thickenFace","extractSolid"]);
function input(feature:TypedCadFeature):string|null|undefined {
 const op=feature.operation;
 if(op.type==="boolean")return typeof op.leftFeatureId==="string"?op.leftFeatureId:undefined;
 if(modifiers.has(op.type))return typeof op.bodyFeatureId==="string"?op.bodyFeatureId:undefined;
 return generators.has(op.type)?null:undefined;
}
function document(project:Project){
 const doc=readTypedCadDocument(project.revisions.find(r=>r.id===project.currentRevision)?.program??"");
 return doc?.revisionId===project.currentRevision?doc:null;
}
export function mirrorRange(project:Project,bodyId:string){
 const doc=document(project),body=doc?.bodies.find(body=>body.id===bodyId);
 if(!doc||!body)return [];
 const chain:TypedCadFeature[]=[],seen=new Set<string>();
 let id:string|null=body.sourceFeatureId;
 while(id&&chain.length<64){
  if(seen.has(id))return [];seen.add(id);
  const feature=doc.features.find(feature=>feature.id===id);if(!feature) return [];
  const parent=input(feature);if(parent===undefined)break;
  chain.unshift(feature);id=parent;
 }
 return chain;
}
export function mirrorBodies(project:Project){return document(project)?.bodies.filter(body=>mirrorRange(project,body.id).length>0)??[];}
export function captureMirrorFeatures(project:Project,bodyId:string,firstId:string,axis:MirrorAxis,offset:number){
 const doc=document(project),body=doc?.bodies.find(body=>body.id===bodyId),chain=mirrorRange(project,bodyId);
 const index=chain.findIndex(feature=>feature.id===firstId);
 if(!doc||!body||index<0||!Number.isFinite(offset)||Math.abs(offset)>10000)return null;
 const selected=chain.slice(index),base=input(selected[0]);if(base===undefined)return null;
 const origin=[0,0,0],normal=[0,0,0],component={x:0,y:1,z:2}[axis];origin[component]=offset;normal[component]=1;
 const feature={id:crypto.randomUUID(),name:t("Mirrored features"),operation:{type:"mirrorFeatures",bodyFeatureId:body.sourceFeatureId,baseFeatureId:base,featureIds:selected.map(f=>f.id),planeOriginMm:origin,planeNormal:normal}};
 const commands=[{command:"add_feature",feature},{command:"set_body_source",bodyId:body.id,sourceFeatureId:feature.id}];
 const captured=captureApply(project,"",t("Mirror features"));
 return {captured,commands,previewSource:JSON.stringify({...doc,features:[...doc.features,feature],bodies:doc.bodies.map(b=>b.id===body.id?{...b,sourceFeatureId:feature.id}:b)}),
 invoke:()=>invoke<Project>("apply_ir_commands",{projectId:captured.base.id,expectedRevision:captured.base.currentRevision,prompt:captured.prompt,commands})};
}
