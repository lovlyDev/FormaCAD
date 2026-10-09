import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
export function modelApplyFixture() {
  const worker = path.resolve("src-tauri/target/debug/forma-cad-worker.exe");
  if (process.platform !== "win32" || !fs.existsSync(worker)) return null;
  const stage = fs.mkdtempSync(path.resolve("../../.local/e2e-model-apply-")), executable = path.join(stage, "forma-cad-worker.exe"); fs.copyFileSync(worker, executable);
  const literal = (mm: number) => ({ kind: "literal", mm });
  const document = { schemaVersion: 2, revisionId: "base", parameters: [], features: [
    { id: "profile", name: "Profile", operation: { type: "rectangle", width: literal(40), depth: literal(20) } },
    { id: "pad", name: "Pad", operation: { type: "extrude", sketchId: "profile", distance: literal(10) } },
    { id: "rotate", name: "Rotate", operation: { type: "rotate", bodyFeatureId: "pad", axisOriginMm: [0, 0, 0], axisDirection: [0, 0, 1], angleDeg: 17 } },
  ], bodies: [{ id: "body", name: "Body", sourceFeatureId: "rotate" }] };
  const assets: Record<string, string> = {}, metadata: Record<string, { size: number; sha256: string }> = {}, metrics: Record<string, any> = {};
  for (const [name, width] of [["base", 40], ["candidate", 52]] as const) {
    const cwd = path.join(stage, name); fs.mkdirSync(cwd);
    const doc = structuredClone(document); doc.revisionId = name; doc.features[0].operation.width = literal(width);
    execFileSync(executable, [], { cwd, input: JSON.stringify({ protocolVersion: 1, requestId: name, document: doc }), windowsHide: true, timeout: 120000,
      env: { ...process.env, PATH: `${path.resolve("../../.local/occt-build/win64/vc14/bin")};${path.resolve("src-tauri/resources/occt")};${process.env.PATH ?? ""}` } });
    metrics[name] = JSON.parse(fs.readFileSync(path.join(cwd, "result.json"), "utf8"));
    if (metrics[name].status !== "completed") throw Error(JSON.stringify(metrics[name]));
    for (const [ext, filename] of [["glb", "preview.glb"], ["step", "model.step"]]) {
      const bytes = fs.readFileSync(path.join(cwd, filename)), key = `${name}.${ext}`;
      assets[key] = `data:${ext === "glb" ? "model/gltf-binary" : "application/step"};base64,${bytes.toString("base64")}`;
      metadata[key] = { size: bytes.length, sha256: crypto.createHash("sha256").update(bytes).digest("hex") };
    }
  }
  const candidate = structuredClone(document); candidate.features[0].operation.width = literal(52);
  const rejected = structuredClone(document) as any;
  rejected.features.push({id:"wrong_branch",name:"Other branch",operation:{type:"rotate",bodyFeatureId:"pad",axisOriginMm:[0,0,0],axisDirection:[0,0,1],angleDeg:29}});
  rejected.features.push({id:"invalid_fillet",name:"Invalid fillet",operation:{type:"filletReferencedEdge",bodyFeatureId:"rotate",radius:literal(1),reference:{schemaVersion:1,kind:"edge",ownerFeatureId:"pad",role:"box-edge:x:ymin:zmin",occurrencePath:["wrong_branch"]}}});
  rejected.bodies[0].sourceFeatureId="invalid_fillet";
  const rejectedStage=path.join(stage,"rejected");fs.mkdirSync(rejectedStage);
  execFileSync(executable,[],{cwd:rejectedStage,input:JSON.stringify({protocolVersion:1,requestId:"rejected",document:rejected}),windowsHide:true,timeout:120000,
    env:{...process.env,PATH:`${path.resolve("../../.local/occt-build/win64/vc14/bin")};${path.resolve("src-tauri/resources/occt")};${process.env.PATH??""}`}});
  const rejection=JSON.parse(fs.readFileSync(path.join(rejectedStage,"result.json"),"utf8"));
  if(rejection.status!=="error"||rejection.code!=="TOPOLOGY_REFERENCE_UNRESOLVED")throw Error(JSON.stringify(rejection));
  if(fs.existsSync(path.join(rejectedStage,"preview.glb"))||fs.existsSync(path.join(rejectedStage,"model.step")))throw Error("Rejected fixture published geometry");
  fs.writeFileSync(path.join(stage,"fixture-verification.json"),JSON.stringify({worker:executable,workerSha256:crypto.createHash("sha256").update(fs.readFileSync(executable)).digest("hex"),metadata,rejection},null,2));
  return { source: JSON.stringify(document), candidate: JSON.stringify(candidate), assets, metadata, metrics, rejection };
}
