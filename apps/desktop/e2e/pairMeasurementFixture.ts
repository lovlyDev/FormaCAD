import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
/** Isolated actual worker assets/reports; browser mocks only the host IPC boundary. */
export function pairMeasurementFixture(){
 const worker=path.resolve("src-tauri/target/debug/forma-cad-worker.exe");if(process.platform!=="win32"||!fs.existsSync(worker))return null;
 const stage=fs.mkdtempSync(path.resolve("../../.local/e2e-pair-measurements-")),executable=path.join(stage,"forma-cad-worker.exe");fs.copyFileSync(worker,executable);
 const digest=(value:crypto.BinaryLike)=>crypto.createHash("sha256").update(value).digest("hex");
 const canonical=(value:any):string=>JSON.stringify(value,(_key,item)=>item&&typeof item==="object"&&!Array.isArray(item)?Object.fromEntries(Object.entries(item).sort(([a],[b])=>a.localeCompare(b))):item);
 const head="c661482b-378a-4fa3-a426-ad9bf6621478",mm=(value:number)=>({kind:"literal",mm:value});
 const document={schemaVersion:2,revisionId:head,parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle",width:mm(40),depth:mm(20)}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:mm(10)}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]},source=JSON.stringify(document);
 const env={...process.env,PATH:`${path.resolve("../../.local/occt-build/win64/vc14/bin")};${process.env.PATH??""}`};
 const run=(cwd:string,input:unknown)=>{execFileSync(executable,[],{cwd,input:JSON.stringify(input),env,windowsHide:true,timeout:120000});const result=JSON.parse(fs.readFileSync(path.join(cwd,"result.json"),"utf8"));if(result.status!=="completed")throw Error(JSON.stringify(result));return result;};
 const base=path.join(stage,"base");fs.mkdirSync(base);const metrics=run(base,{protocolVersion:1,requestId:crypto.randomUUID(),document});
 const assets:Record<string,string>={},metadata:Record<string,{size:number;sha256:string}>={};
 for(const [name,file,mime] of [["base.step","model.step","application/step"],["base.glb","preview.glb","model/gltf-binary"]]){const bytes=fs.readFileSync(path.join(base,file));metadata[name]={size:bytes.length,sha256:digest(bytes)};assets[name]=`data:${mime};base64,${bytes.toString("base64")}`;}
 const reference=(kind:string,role:string)=>({schemaVersion:1,kind,ownerFeatureId:"pad",role,occurrencePath:[]});
 const queries:any[]=[];
 for(const sign of ["min","max"]){const lower=reference("edge",`box-edge:x:y${sign}:zmin`),upper=reference("edge",`box-edge:x:y${sign}:zmax`);queries.push({kind:"minimumDistance",first:lower,second:upper},{kind:"minimumDistance",first:upper,second:lower},{kind:"edgeAcuteAngle",first:lower,second:upper},{kind:"edgeAcuteAngle",first:upper,second:lower});}
 const top=reference("face","box-face:zmax");queries.push({kind:"minimumDistance",first:top,second:top});
 for(const role of ["box-face:xmin","box-face:xmax","box-face:ymin","box-face:ymax"]){const side=reference("face",role);queries.push({kind:"faceNormalAngle",first:top,second:side});}
 const reports:Record<string,any>={};
 for(const [index,query] of queries.entries()){const cwd=path.join(stage,String(index));fs.mkdirSync(cwd);fs.writeFileSync(path.join(cwd,"document.json"),source);fs.copyFileSync(path.join(base,"model.step"),path.join(cwd,"source.step"));const response=run(cwd,{protocolVersion:1,requestId:crypto.randomUUID(),operation:"measure_pair",bodyId:"body",documentSha256:digest(source),sourceSha256:metadata["base.step"].sha256,sourceSize:metadata["base.step"].size,pairMeasurementQuery:query});const bytes=fs.readFileSync(path.join(cwd,"pair-measurement.json"));if(digest(bytes)!==response.measurementSha256)throw Error("Pair report checksum mismatch");const report=JSON.parse(bytes.toString("utf8"));if(query.kind==="minimumDistance"&&Math.abs(report.result.distanceMm-(query.first.kind==="face"?0:10))>1e-7)throw Error("Independent box distance mismatch");if(query.kind==="faceNormalAngle"&&Math.abs(report.result.angleDeg-90)>1e-7)throw Error("Independent box outward angle mismatch");if(query.kind==="edgeAcuteAngle"&&Math.abs(report.result.angleDeg)>1e-7)throw Error("Independent parallel edge angle mismatch");reports[canonical(query)]=report;}
 fs.writeFileSync(path.join(stage,"fixture-verification.json"),JSON.stringify({workerSha256:digest(fs.readFileSync(executable)),metadata,reports},null,2));
 return{head,source,candidate:source,assets,metadata,metrics:{base:metrics},rejection:{},reports:{},pairReports:reports};
}


