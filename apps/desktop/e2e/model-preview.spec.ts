import { expect,test } from "@playwright/test";
import fs from "node:fs";import path from "node:path";import crypto from "node:crypto";
import { nativePreviewFixture } from "./nativePreviewFixture";
const nativeFixture=nativePreviewFixture();
const stage=nativeFixture?.stage??"";
const source=fs.readFileSync(path.resolve("../../docs/fixtures/parameterized-disc.cad.json"),"utf8");
test("native preview UI uses exact worker geometry without changing project history",async({page},info)=>{
 test.skip(!nativeFixture,"Requires the locally built Windows native CAD worker");
 await page.goto("/");await expect(page.locator("html")).toHaveAttribute("lang","ru");
 let calls=0;
 await page.route("**/__preview_model",async route=>{
  const request=route.request().postDataJSON();const program=JSON.parse(request.program);const height=program.parameters.find((p:{id:string})=>p.id==="thickness").valueMm;
  const result=JSON.parse(fs.readFileSync(path.join(stage,`${height}/result.json`),"utf8"));
  const header=Buffer.from(JSON.stringify({protocolVersion:1,sourceSha256:crypto.createHash("sha256").update(request.program).digest("hex"),metrics:{volumeMm3:result.volumeMm3,areaMm2:result.areaMm2,faceCount:result.faceCount,edgeCount:result.edgeCount,boundsMm:result.boundsMm}}));
  const prefix=Buffer.alloc(4);prefix.writeUInt32LE(header.length);calls++;
  await route.fulfill({contentType:"application/octet-stream",body:Buffer.concat([prefix,header,fs.readFileSync(path.join(stage,`${height}/preview.glb`))])});
 });
 await page.evaluate(async source=>{
  const React=await import("/node_modules/.vite/deps/react.js");const ReactDOM=await import("/node_modules/.vite/deps/react-dom_client.js");
  const {ProgramDialog}=await import("/src/app/dialogs/ProgramDialog.tsx");const {useWorkspace,newProject}=await import("/src/stores/workspace.ts");
  const project=newProject("Preview test","blank","mm","codex");useWorkspace.getState().setProject(project);
  window.isTauri=true;
  window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:"main"},currentWebview:{label:"main"}},invoke:async(command:string,args:unknown)=>command==="preview_model"?(await fetch("/__preview_model",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(args)})).arrayBuffer():null};
  const root=ReactDOM.default.createRoot(document.body.appendChild(document.createElement("div")));
  root.render(React.default.createElement(ProgramDialog,{open:true,close:()=>root.unmount(),source,disabled:false,nativeCadAvailable:true,nativeCadChecked:true,onApply:()=>{throw new Error("Preview must not commit");}}));
 },source);
 await page.getByRole("button",{name:"Предпросмотр модели",exact:true}).click();
 await expect(page.locator(".model-preview-stage canvas")).toBeVisible();await expect(page.locator(".model-preview-metrics")).toContainText("21");expect(calls).toBe(1);
 const previewCanvas=page.locator(".model-preview-stage canvas");await expect.poll(async()=>JSON.parse(await previewCanvas.getAttribute("data-preview-camera")??"{}").moving).toBe(false);
 const bounds=(await previewCanvas.boundingBox())!;await page.mouse.move(bounds.x+bounds.width/2,bounds.y+bounds.height/2);await page.mouse.down({button:"right"});await page.mouse.move(bounds.x+bounds.width/2+60,bounds.y+bounds.height/2+20,{steps:10});await page.mouse.up({button:"right"});
 const pan=JSON.parse(await previewCanvas.getAttribute("data-preview-camera")??"{}");await page.getByRole("button",{name:"Вписать предпросмотр в окно",exact:true}).click();
 const samples=await previewCanvas.evaluate(async element=>{const frames=[];for(let n=0;n<35;n++){await new Promise<void>(resolve=>requestAnimationFrame(()=>resolve()));frames.push(JSON.parse((element as HTMLCanvasElement).dataset.previewCamera!));}return frames;});
 expect(samples.some(sample=>sample.moving)).toBe(true);const fitted=samples.at(-1)!;expect(fitted.moving).toBe(false);let previous=Infinity;for(const sample of samples){const distance=Math.hypot(...sample.target.map((value:number,i:number)=>value-fitted.target[i]));expect(distance).toBeLessThanOrEqual(previous+1e-6);previous=distance;}expect(Math.hypot(...pan.target.map((value:number,i:number)=>value-fitted.target[i]))).toBeGreaterThan(.1);
 await page.getByLabel("Параметр thickness в мм",{exact:true}).fill("12");await expect.poll(()=>calls).toBe(2);await expect(page.locator(".model-preview-stage canvas")).toBeVisible();
 for(const theme of ["dark","light"]){await page.evaluate(async theme=>{const {applyTheme}=await import("/src/lib/theme.ts");applyTheme(theme);},theme);await page.locator(".model-editor-scroll").evaluate(element=>element.scrollTop=0);await page.locator(".model-editor-modal").screenshot({path:info.outputPath(`preview-ru-${theme}.png`)});}
 expect(await page.evaluate(async()=>{const {useWorkspace}=await import("/src/stores/workspace.ts");return useWorkspace.getState().project!.revisions.length;})).toBe(0);
 await page.getByRole("button",{name:"Закрыть",exact:true}).click();await expect(page.locator(".model-editor-modal")).toHaveCount(0);
});
