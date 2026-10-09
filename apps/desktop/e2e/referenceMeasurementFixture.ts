import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";

/** Genuine isolated worker reports; the browser mocks only the host IPC boundary. */
export function referenceMeasurementFixture() {
  const worker=path.resolve("src-tauri/target/debug/forma-cad-worker.exe");
  if(process.platform!=="win32"||!fs.existsSync(worker))return null;
  const stage=fs.mkdtempSync(path.resolve("../../.local/e2e-reference-measurements-"));
  const executable=path.join(stage,"forma-cad-worker.exe");fs.copyFileSync(worker,executable);
  const digest=(bytes:crypto.BinaryLike)=>crypto.createHash("sha256").update(bytes).digest("hex");
  const head="c661482b-378a-4fa3-a426-ad9bf6621478",literal=(mm:number)=>({kind:"literal",mm});
  const document={schemaVersion:2,revisionId:head,parameters:[],features:[
    {id:"profile",name:"Profile",operation:{type:"rectangle",width:literal(40),depth:literal(20)}},
    {id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:literal(10)}}
  ],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]};
  const source=JSON.stringify(document),env={...process.env,PATH:`${path.resolve("../../.local/occt-build/win64/vc14/bin")};${process.env.PATH??""}`};
  const run=(cwd:string,input:unknown)=>{execFileSync(executable,[],{cwd,input:JSON.stringify(input),env,windowsHide:true,timeout:120000});return JSON.parse(fs.readFileSync(path.join(cwd,"result.json"),"utf8"));};
  const base=path.join(stage,"base");fs.mkdirSync(base);
  const metrics=run(base,{protocolVersion:1,requestId:crypto.randomUUID(),document});
  if(metrics.status!=="completed")throw Error(JSON.stringify(metrics));
  const assets:Record<string,string>={},metadata:Record<string,{size:number;sha256:string}>={};
  for(const [name,file,mime] of [["base.step","model.step","application/step"],["base.glb","preview.glb","model/gltf-binary"]]){
    const bytes=fs.readFileSync(path.join(base,file));metadata[name]={size:bytes.length,sha256:digest(bytes)};assets[name]=`data:${mime};base64,${bytes.toString("base64")}`;
  }
  const edge={schemaVersion:1,kind:"edge",ownerFeatureId:"pad",role:"box-edge:x:ymin:zmin",occurrencePath:[]};
  const face={schemaVersion:1,kind:"face",ownerFeatureId:"pad",role:"box-face:zmax",occurrencePath:[]};
  const queries=[{kind:"bodyMetrics"},{kind:"edgeLength",reference:edge},{kind:"faceArea",reference:face},{kind:"planarFace",reference:face}];
  const reports:Record<string,any>={};
  for(const query of queries){
    const cwd=path.join(stage,query.kind);fs.mkdirSync(cwd);fs.writeFileSync(path.join(cwd,"document.json"),source);fs.copyFileSync(path.join(base,"model.step"),path.join(cwd,"source.step"));
    const response=run(cwd,{protocolVersion:1,requestId:crypto.randomUUID(),operation:"measure_reference",bodyId:"body",documentSha256:digest(source),sourceSha256:metadata["base.step"].sha256,sourceSize:metadata["base.step"].size,measurementQuery:query});
    if(response.status!=="completed")throw Error(JSON.stringify(response));
    const raw=fs.readFileSync(path.join(cwd,"measurement.json"));if(digest(raw)!==response.measurementSha256)throw Error("Measurement report checksum mismatch");
    reports[query.kind]=JSON.parse(raw.toString("utf8"));
  }
  if(Math.abs(reports.bodyMetrics.result.volumeMm3-8000)>1e-6||Math.abs(reports.edgeLength.result.lengthMm-40)>1e-6||Math.abs(reports.faceArea.result.areaMm2-800)>1e-6)throw Error("Analytical measurement fixture mismatch");
  fs.writeFileSync(path.join(stage,"fixture-verification.json"),JSON.stringify({worker:executable,workerSha256:digest(fs.readFileSync(executable)),metadata,reports},null,2));
  return{head,source,candidate:source,assets,metadata,metrics:{base:metrics},rejection:{},reports,edge,face};
}
