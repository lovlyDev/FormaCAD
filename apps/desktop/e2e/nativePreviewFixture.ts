import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
/** Actual local worker outputs; no committed binary fixture or historical .local path dependency. */
export function nativePreviewFixture() {
 const worker=path.resolve("src-tauri/binaries/forma-cad-worker-x86_64-pc-windows-msvc.exe");
 if(process.platform!=="win32"||!fs.existsSync(worker))return null;
 const document=JSON.parse(fs.readFileSync(path.resolve("../../docs/fixtures/parameterized-disc.cad.json"),"utf8"));
 const root=path.resolve("../../.local");fs.mkdirSync(root,{recursive:true});const stage=fs.mkdtempSync(path.join(root,"e2e-native-preview-"));
 const isolatedWorker=path.join(stage,"forma-cad-worker.exe");fs.copyFileSync(worker,isolatedWorker);
 for(const height of [8,12]) {
  document.parameters.find((p:{id:string})=>p.id==="thickness").valueMm=height;
  const cwd=path.join(stage,String(height));fs.mkdirSync(cwd);
  execFileSync(isolatedWorker,[],{cwd,input:JSON.stringify({protocolVersion:1,requestId:String(height),document}),env:{...process.env,PATH:`${path.resolve("src-tauri/resources/occt")};${process.env.PATH??""}`},stdio:["pipe","pipe","pipe"],windowsHide:true,timeout:120000});
  const result=JSON.parse(fs.readFileSync(path.join(cwd,"result.json"),"utf8"));if(result.status!=="completed")throw new Error(JSON.stringify(result));
 }
 document.parameters.find((p:{id:string})=>p.id==="thickness").valueMm=8;
 return {stage,document,glb:`data:model/gltf-binary;base64,${fs.readFileSync(path.join(stage,"8/preview.glb")).toString("base64")}`};
}
