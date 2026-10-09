import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
export function faceSectionFixture(){
 const worker=path.resolve("src-tauri/target/debug/forma-cad-worker.exe");if(process.platform!=="win32"||!fs.existsSync(worker))return null;
 const stage=fs.mkdtempSync(path.resolve("../../.local/e2e-face-section-")),executable=path.join(stage,"forma-cad-worker.exe");fs.copyFileSync(worker,executable);
 const digest=(value:crypto.BinaryLike)=>crypto.createHash("sha256").update(value).digest("hex"),canonical=(value:any)=>JSON.stringify(value,(_key,item)=>item&&typeof item==="object"&&!Array.isArray(item)?Object.fromEntries(Object.entries(item).sort(([a],[b])=>a.localeCompare(b))):item);
 const head="c661482b-378a-4fa3-a426-ad9bf6621478",mm=(value:number)=>({kind:"literal",mm:value});
 const document={schemaVersion:2,revisionId:head,parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle",width:mm(40),depth:mm(20)}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:mm(10)}},{id:"circle",name:"Circle",operation:{type:"circle",radius:mm(8)}},{id:"circle_pad",name:"Cylinder",operation:{type:"extrude",sketchId:"circle",distance:mm(10)}},{id:"moved",name:"Moved",operation:{type:"translate",bodyFeatureId:"circle_pad",offsetMm:[65,0,0]}}],bodies:[{id:"body",name:"Box",sourceFeatureId:"pad"},{id:"second",name:"Cylinder",sourceFeatureId:"moved"}]};
 const source=JSON.stringify(document),env={...process.env,PATH:`${path.resolve("../../.local/occt-build/win64/vc14/bin")};${process.env.PATH??""}`};
 const run=(cwd:string,input:unknown)=>{execFileSync(executable,[],{cwd,input:JSON.stringify(input),env,windowsHide:true,timeout:120000});const result=JSON.parse(fs.readFileSync(path.join(cwd,"result.json"),"utf8"));if(result.status!=="completed")throw Error(JSON.stringify(result));return result;};
 const base=path.join(stage,"base");fs.mkdirSync(base);const metrics=run(base,{protocolVersion:1,requestId:crypto.randomUUID(),document});
 const assets:Record<string,string>={},metadata:Record<string,{size:number;sha256:string}>={};
 for(const[name,file,mime]of[["base.step","model.step","application/step"],["base.glb","preview.glb","model/gltf-binary"]]){const value=fs.readFileSync(path.join(base,file));metadata[name]={size:value.length,sha256:digest(value)};assets[name]=`data:${mime};base64,${value.toString("base64")}`;}
 const face={schemaVersion:1,kind:"face",ownerFeatureId:"pad",role:"box-face:zmax",occurrencePath:[]},edge={...face,kind:"edge",role:"box-edge:x:ymin:zmax"};const faceReports:Record<string,any>={};
 for(const offsetMm of [0,-2,-5,6]){const query={reference:face,offsetMm,deflectionMm:.01},cwd=path.join(stage,`face-${offsetMm}`);fs.mkdirSync(cwd);fs.writeFileSync(path.join(cwd,"document.json"),source);fs.copyFileSync(path.join(base,"model.step"),path.join(cwd,"source.step"));
  const response=run(cwd,{protocolVersion:1,requestId:crypto.randomUUID(),operation:"section_reference",bodyId:"body",documentSha256:digest(source),sourceSha256:metadata["base.step"].sha256,sourceSize:metadata["base.step"].size,faceSectionQuery:query});const bytes=fs.readFileSync(path.join(cwd,"face-section.json"));if(digest(bytes)!==response.sectionSha256)throw Error("Face-section checksum mismatch");faceReports[canonical(query)]=JSON.parse(bytes.toString("utf8"));
 }
 const intersection=Object.values(faceReports).find(report=>report.query.offsetMm===-2),empty=Object.values(faceReports).find(report=>report.query.offsetMm===6);if(Math.abs(intersection.geometry.totalLengthMm-(120+16*Math.PI))>1e-6||empty.geometry.curves.length||empty.geometry.totalLengthMm!==0)throw Error("Analytical whole-STEP fixture mismatch");
 const cwd=path.join(stage,"manual");fs.mkdirSync(path.join(cwd,"inputs"),{recursive:true});fs.copyFileSync(path.join(base,"model.step"),path.join(cwd,"inputs",`step-${metadata["base.step"].sha256}.step`));const response=run(cwd,{protocolVersion:1,requestId:crypto.randomUUID(),operation:"section_step",sourceSha256:metadata["base.step"].sha256,sectionPlane:{originMm:[0,0,5],normal:[0,0,1],deflectionMm:.01}});const manualBytes=fs.readFileSync(path.join(cwd,"section.json"));if(digest(manualBytes)!==response.sectionSha256)throw Error("Manual-section checksum mismatch");const manualReport=JSON.parse(manualBytes.toString("utf8"));
 fs.writeFileSync(path.join(stage,"fixture-verification.json"),JSON.stringify({worker:executable,workerSha256:digest(fs.readFileSync(executable)),metadata,faceReports,manualReport},null,2));
 return{head,source,candidate:source,assets,metadata,metrics:{base:metrics},rejection:{},reports:{},edge,face,faceReports,manualReport};
}
