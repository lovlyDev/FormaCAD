import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";

/** Real isolated OCCT build/reports. Host IPC is mocked by the browser harness. */
export function circularMeasurementFixture(){
 const worker=path.resolve("src-tauri/target/debug/forma-cad-worker.exe");
 if(process.platform!=="win32"||!fs.existsSync(worker))return null;
 const stage=fs.mkdtempSync(path.resolve("../../.local/e2e-circular-measurements-")),executable=path.join(stage,"forma-cad-worker.exe");fs.copyFileSync(worker,executable);
 const digest=(value:crypto.BinaryLike)=>crypto.createHash("sha256").update(value).digest("hex");
 const canonical=(value:any)=>JSON.stringify(value,(_key,item)=>item&&typeof item==="object"&&!Array.isArray(item)?Object.fromEntries(Object.entries(item).sort(([a],[b])=>a.localeCompare(b))):item);
 const head="c661482b-378a-4fa3-a426-ad9bf6621478",mm=(value:number)=>({kind:"literal",mm:value});
 const document={schemaVersion:2,revisionId:head,parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"circle",radius:mm(25.4)}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:mm(10)}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]};
 const source=JSON.stringify(document),env={...process.env,PATH:`${path.resolve("../../.local/occt-build/win64/vc14/bin")};${process.env.PATH??""}`};
 const run=(cwd:string,input:unknown)=>{execFileSync(executable,[],{cwd,input:JSON.stringify(input),env,windowsHide:true,timeout:120000});return JSON.parse(fs.readFileSync(path.join(cwd,"result.json"),"utf8"));};
 const base=path.join(stage,"base");fs.mkdirSync(base);const metrics=run(base,{protocolVersion:1,requestId:crypto.randomUUID(),document});if(metrics.status!=="completed")throw Error(JSON.stringify(metrics));
 const assets:Record<string,string>={},metadata:Record<string,{size:number;sha256:string}>={};
 for(const [name,file,mime] of [["base.step","model.step","application/step"],["base.glb","preview.glb","model/gltf-binary"]]){const value=fs.readFileSync(path.join(base,file));metadata[name]={size:value.length,sha256:digest(value)};assets[name]=`data:${mime};base64,${value.toString("base64")}`;}
 const edge={schemaVersion:1,kind:"edge",ownerFeatureId:"pad",role:"cylinder-edge:top",occurrencePath:[]};
 const face={schemaVersion:1,kind:"face",ownerFeatureId:"pad",role:"cylinder-face:top",occurrencePath:[]},side={...face,role:"cylinder-face:side"};
 const queries=[{kind:"bodyMetrics"},{kind:"edgeLength",reference:edge},{kind:"edgeRadius",reference:edge},{kind:"edgeDiameter",reference:edge},{kind:"faceArea",reference:face},{kind:"planarFace",reference:face},{kind:"faceArea",reference:side}];
 const reports:Record<string,any>={};
 for(const [index,query] of queries.entries()){const cwd=path.join(stage,String(index));fs.mkdirSync(cwd);fs.writeFileSync(path.join(cwd,"document.json"),source);fs.copyFileSync(path.join(base,"model.step"),path.join(cwd,"source.step"));
  const response=run(cwd,{protocolVersion:1,requestId:crypto.randomUUID(),operation:"measure_reference",bodyId:"body",documentSha256:digest(source),sourceSha256:metadata["base.step"].sha256,sourceSize:metadata["base.step"].size,measurementQuery:query});if(response.status!=="completed")throw Error(JSON.stringify(response));
  const raw=fs.readFileSync(path.join(cwd,"measurement.json"));if(digest(raw)!==response.measurementSha256)throw Error("Measurement checksum mismatch");reports[canonical(query)]=JSON.parse(raw.toString("utf8"));
 }
 const result=(kind:string)=>Object.values(reports).find(report=>report.query.kind===kind).result;
 if(Math.abs(result("edgeRadius").radiusMm-25.4)>1e-8||Math.abs(result("edgeDiameter").diameterMm-50.8)>1e-8||Math.abs(result("edgeLength").lengthMm-2*Math.PI*25.4)>1e-6)throw Error("Circular analytical fixture mismatch");
 fs.writeFileSync(path.join(stage,"fixture-verification.json"),JSON.stringify({worker:executable,workerSha256:digest(fs.readFileSync(executable)),metadata,reports},null,2));
 return{head,source,candidate:source,assets,metadata,metrics:{base:metrics},rejection:{},reports,edge,face,side};
}
