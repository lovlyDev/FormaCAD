/** Real built-app UI → IPC → host → OCCT acceptance. Isolated profile required. No IPC mocks or failed-call retries. */
import fs from "node:fs";
import path from "node:path";
import {randomUUID,createHash} from "node:crypto";
import {chromium,expect as baseExpect} from "@playwright/test";
import {spawn} from "node:child_process";
import {foregroundPixels} from "./canvasPixels.mjs";
const expect=baseExpect.configure({timeout:30000});
const [endpoint,output]=process.argv.slice(2);
if(process.env.FORMA_NATIVE_ACCEPTANCE!=="isolated"||endpoint!=="http://127.0.0.1:9336"||!output)throw Error("Use an isolated native host and an output report path");
const report={ipcMocked:false,setup:"real native fixture commands; visible thumbnail before snapshot reads",cases:[],errors:[]};
let browser,page;
const invoke=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
async function projectById(id){return (await invoke("list_projects")).find(project=>project.id===id);}
const cameraKey=id=>`forma.ui.project.${id}.view.main.camera`;
async function settledCamera(id){
 let last="",stable=0;
 await expect.poll(async()=>{
  const value=await page.evaluate(key=>localStorage.getItem(key),cameraKey(id));
  stable=value&&value===last?stable+1:0;last=value??"";return stable;
 }).toBeGreaterThan(2);
}
async function home(){if(await page.locator(".viewer-area").count()){const locale=await page.locator("html").getAttribute("lang");await expect(page.getByRole("button",{name:tr(locale,"Export"),exact:true}).first()).toBeEnabled();await page.locator("button.brand").click();}await expect(page.locator(".dashboard")).toBeVisible();}
async function seed(name,kind){
 const now=new Date().toISOString(),project={schemaVersion:1,id:randomUUID(),name:name+" "+randomUUID().slice(0,8),units:"mm",agent:"codex",pinned:false,createdAt:now,updatedAt:now,revisions:[],currentRevision:null,messages:[],files:[],exports:[]};
 await invoke("save_project",{project});
 const id=await invoke("request_permission",{projectId:project.id,action:"modify_project",detail:"Isolated IGES acceptance fixture"});
 await invoke("resolve_permission",{id,allow:true});
 const literal=mm=>({kind:"literal",mm});
 const document={schemaVersion:2,revisionId:"candidate",parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle",width:literal(40),depth:literal(20)}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:literal(10)}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]};
 if(kind==="compound"){
  document.features.push({id:"circle",name:"Circle",operation:{type:"circle",radius:literal(3)}},{id:"cylinder",name:"Cylinder",operation:{type:"extrude",sketchId:"circle",distance:literal(10)}},{id:"moved",name:"Moved",operation:{type:"translate",bodyFeatureId:"cylinder",offsetMm:[60,0,0]}});
  document.bodies.push({id:"cylinder_body",name:"Cylinder",sourceFeatureId:"moved"});
 }
 if(kind==="variable"){
  document.features.push({id:"rounded",name:"Rounded",operation:{type:"variableFillet",bodyFeatureId:"pad",reference:{schemaVersion:1,kind:"edge",ownerFeatureId:"pad",role:"box-edge:z:xmax:ymin",occurrencePath:[]},radiusStart:literal(1.25),radiusEnd:literal(3.5)}});
  document.bodies[0].sourceFeatureId="rounded";
 }
 const created=await invoke("apply_program",{projectId:project.id,program:JSON.stringify(document),prompt:"IGES acceptance baseline",expectedRevision:null});
 await expect(page.locator(".project-card").filter({has:page.locator(".project-card-info > button").filter({hasText:created.name})}).locator(".project-art img")).toBeVisible();
 return created;
}
async function openProject(project){
 await page.reload();await expect(page.locator(".dashboard")).toBeVisible();
 await page.locator(".project-card-info > button").filter({hasText:project.name}).click();
 await expect(page.locator(".viewer-area")).toHaveAttribute("data-displayed-revision",project.currentRevision);
 await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
}
const runtime=path.resolve(path.dirname(output));
const dictionaries=Object.fromEntries(["ru","en"].map(locale=>[locale,JSON.parse(fs.readFileSync(new URL(`../../apps/desktop/src/i18n/${locale}.json`,import.meta.url),"utf8"))]));
const tr=(locale,key)=>{const text=dictionaries[locale][key];if(!text)throw Error(`Missing ${locale} catalog key: ${key}`);return text;};
async function approve(ru){await page.getByRole("button",{name:ru?"Разрешить один раз":"Allow once",exact:true}).click();}
async function select(dialog,index,value){
 const trigger=dialog.getByRole("combobox").nth(index);await trigger.click();
 await page.locator(`.select-menu[data-state="open"] .select-option[data-value="${value}"]`).click();
 await expect(trigger).toHaveAttribute("data-state","closed");
}
let saveChild;
function nativeSave(destination,cancel=false){
 const hostId=fs.readFileSync(path.join(runtime,"pid.txt"),"utf8").trim();
 const script=path.resolve("scripts/acceptance/iges-native-save.ps1");
 const child=spawn("powershell.exe",["-NoProfile","-NonInteractive","-ExecutionPolicy","Bypass","-File",script,"-HostProcessId",hostId,"-RuntimeDirectory",runtime,"-Destination",destination,...(cancel?["-Cancel"]:[])],{windowsHide:true,env:process.env});
 saveChild=child;let log="";
 child.stdout.on("data",data=>{log+=data;});child.stderr.on("data",data=>{log+=data;});
 return new Promise(resolve=>{child.on("error",error=>resolve({error:String(error)}));child.on("close",code=>resolve(code===0?{log}:{error:log,code}));});
}
async function previewDialog(locale){
 const dialog=page.getByRole("dialog",{name:tr(locale,"Import IGES model"),exact:true});
 await expect(dialog.getByText(tr(locale,"IGES round-trip verified. All closed solids are preserved."),{exact:true})).toBeVisible();
 const canvas=dialog.locator(".model-preview-canvas canvas");await canvas.hover();await page.bringToFront();
 await expect.poll(async()=>foregroundPixels(await canvas.screenshot())).toBeGreaterThan(200);
 return dialog;
}
async function openExporter(locale,kind,scope){
 await page.getByRole("button",{name:tr(locale,"Export"),exact:true}).first().click();
 const dialog=page.getByRole("dialog",{name:tr(locale,"Ready for the next step"),exact:true});
 await select(dialog,0,"iges");if(kind==="compound")await select(dialog,1,scope);
 await expect(dialog.getByText(tr(locale,"Exact solid"),{exact:true})).toBeVisible();return dialog;
}
async function liveThemes(project,locale,theme){
 const canvas=await page.locator(".viewer-area canvas").elementHandle();await settledCamera(project.id);
 const camera=await page.evaluate(key=>localStorage.getItem(key),cameraKey(project.id));
 await page.getByRole("button",{name:tr(locale,"Settings"),exact:true}).first().click();
 await page.getByRole("button",{name:tr(locale,"Appearance"),exact:true}).click();
 const settings=page.getByRole("dialog",{name:tr(locale,"Settings"),exact:true});
 for(const next of [theme==="dark"?"light":"dark",theme]){
  await select(settings,1,next);await expect(page.locator("html")).toHaveAttribute("data-theme",next);
  expect(await canvas.evaluate(node=>node.isConnected)).toBe(true);
  await expect(page.locator(".viewer-area")).toHaveAttribute("data-displayed-revision",project.currentRevision);
  expect(await page.evaluate(key=>localStorage.getItem(key),cameraKey(project.id))).toBe(camera);
 }
 await settings.locator(".modal-close").click();await expect(settings).toHaveCount(0);
}
try{
 if(process.platform!=="win32")throw Error("This scenario exercises the actual Windows save dialog");
 browser=await chromium.connectOverCDP(endpoint);
 page=browser.contexts().flatMap(context=>context.pages()).find(page=>/tauri\.localhost/.test(page.url()));
 if(!page)throw Error("Built Tauri page missing");
 page.setDefaultTimeout(30000);page.on("pageerror",error=>report.errors.push({message:error.message,stack:error.stack}));
 if(await page.getByRole("dialog").count())await page.getByRole("dialog").locator(".modal-close").click();
 await home();
 for(const locale of ["ru","en"])for(const theme of ["dark","light"]){
  const ru=locale==="ru",kind=theme==="light"&&ru||theme==="dark"&&!ru?"compound":theme==="light"&&!ru?"variable":"box";
  const scope=ru&&theme==="light"?"body":"",item={locale,theme,kind,scope:scope||"all"};report.cases.push(item);
  await home();const preferences=await invoke("get_ui_preferences");
  await invoke("set_ui_preferences",{value:{...preferences,"forma.locale":locale,"forma.theme":theme}});
  await page.evaluate(({locale,theme})=>{localStorage.setItem("forma.locale",locale);localStorage.setItem("forma.theme",theme);},{locale,theme});
  await page.reload();await expect(page.locator("html")).toHaveAttribute("lang",locale);await expect(page.locator("html")).toHaveAttribute("data-theme",theme);
  const baseline=await seed(`M4 IGES ${locale} ${theme}`,kind);await openProject(baseline);
  item.before=await invoke("inspect_model",{projectId:baseline.id,revisionId:baseline.currentRevision});
  if(ru&&theme==="dark"){
   const beforeCancel=await projectById(baseline.id),dialog=await openExporter(locale,kind,scope);
   await dialog.getByRole("button",{name:`${tr(locale,"Export")} IGES`,exact:true}).click();
   const canceled=nativeSave(path.join(runtime,`canceled-${randomUUID()}.iges`),true);await approve(ru);
   const result=await canceled;if(result.error)throw Error(result.error);
   await expect(page.getByRole("button",{name:tr(locale,"Export"),exact:true}).first()).toBeEnabled();
   expect(await projectById(baseline.id)).toEqual(beforeCancel);item.nativeSaveCancelKeptProject=true;
  }
  const exporter=await openExporter(locale,kind,scope);
  const filename=path.join(runtime,`${baseline.name.replace(/[^\p{L}\p{N}_-]/gu,"-")}-r${baseline.revisions.length}.iges`);
  await exporter.getByRole("button",{name:`${tr(locale,"Export")} IGES`,exact:true}).click();
  const saved=nativeSave(filename);await approve(ru);const result=await saved;if(result.error)throw Error(result.error);
  item.saveDialog=result.log;
  const receipt=page.getByRole("dialog",{name:tr(locale,"IGES export verified"),exact:true});await expect(receipt).toBeVisible();
  await expect(receipt.locator(".iges-exchange-path")).toHaveText(filename);
  item.exportTable=await receipt.locator("table").innerText();
  await receipt.screenshot({path:path.join(runtime,`iges-export-${locale}-${theme}.png`)});
  await receipt.getByRole("button",{name:tr(locale,"Close"),exact:true}).click();await expect(receipt).toHaveCount(0);
  await expect(page.getByRole("button",{name:tr(locale,"Export"),exact:true}).first()).toBeEnabled();
  const current=await projectById(baseline.id);expect(current.currentRevision).toBe(baseline.currentRevision);
  const bytes=fs.readFileSync(filename),digest=createHash("sha256").update(bytes).digest("hex");
  const checked=await invoke("preview_iges_import",{request:{projectId:current.id,expectedRevision:current.currentRevision,name:path.basename(filename),data:`data:application/octet-stream;base64,${bytes.toString("base64")}`}});
  item.exchange=checked.report;expect(checked.report.sourceSha256).toBe(digest);
  const expectedVolume=kind==="variable"?7986.765923888611:kind==="compound"&&!scope?8000+90*Math.PI:8000;
  const expectedArea=kind==="variable"?2786.932710141804:kind==="compound"&&!scope?2800+78*Math.PI:2800;
  expect(checked.report.source.volumeMm3).toBeCloseTo(expectedVolume,3);expect(checked.report.source.areaMm2).toBeCloseTo(expectedArea,3);
  expect(checked.report.source.solidCount).toBe(kind==="compound"&&!scope?2:1);
  const fileInput=page.locator('input[type="file"][accept*=".iges"]');
  await fileInput.setInputFiles({name:"invalid.igs",mimeType:"application/octet-stream",buffer:Buffer.from("invalid IGES")});
  let dialog=page.getByRole("dialog",{name:tr(locale,"Import IGES model"),exact:true});
  await expect(dialog.getByRole("alert")).toBeVisible();await expect(dialog.getByRole("alert")).not.toContainText(/IGES_[A-Z_]+/);
  await expect(dialog.getByRole("button",{name:tr(locale,"Import IGES"),exact:true})).toBeDisabled();
  item.invalidMessage=await dialog.getByRole("alert").innerText();
  await dialog.getByRole("button",{name:tr(locale,"Cancel"),exact:true}).click();
  expect((await projectById(current.id))).toEqual(current);
  await fileInput.setInputFiles(filename);dialog=await previewDialog(locale);
  await expect(dialog.getByRole("button",{name:tr(locale,"Import IGES"),exact:true})).toBeDisabled();
  await dialog.getByRole("button",{name:tr(locale,"Cancel"),exact:true}).click();expect(await projectById(current.id)).toEqual(current);
  await fileInput.setInputFiles(filename);dialog=await previewDialog(locale);
  await dialog.getByRole("checkbox").check();await dialog.screenshot({path:path.join(runtime,`iges-import-${locale}-${theme}.png`)});
  await dialog.getByRole("button",{name:tr(locale,"Import IGES"),exact:true}).click();await approve(ru);
  await expect(page.locator(".viewer-area")).not.toHaveAttribute("data-displayed-revision",current.currentRevision);
  await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
  const committed=await projectById(current.id),revision=committed.revisions.find(r=>r.id===committed.currentRevision),doc=JSON.parse(revision.program);
  expect(committed.revisions.length).toBe(current.revisions.length+1);expect(committed.revisions.find(r=>r.id===current.currentRevision).program).toBe(current.revisions.at(-1).program);
  expect(doc.features[0].id).toBe("imported_iges");expect(doc.bodies[0].id).toBe("imported_body");
  expect(committed.files.find(f=>f.name===path.basename(filename)).sha256).toBe(digest);
  item.properties=await invoke("inspect_model",{projectId:committed.id,revisionId:committed.currentRevision});
  item.body=await invoke("measure_model_reference",{projectId:committed.id,expectedRevision:committed.currentRevision,bodyId:"imported_body",query:{kind:"bodyMetrics"}});
  expect(item.properties.volumeMm3).toBeCloseTo(expectedVolume,3);expect(item.properties.areaMm2).toBeCloseTo(expectedArea,3);expect(item.body.result.volumeMm3).toBeCloseTo(expectedVolume,3);
  item.history={};
  for(const [direction,index,volume] of [["undo",0,item.before.volumeMm3],["redo",1,expectedVolume]]){
   const previous=(await projectById(current.id)).currentRevision;
   await page.locator(".model-history-controls button").nth(index).click();await approve(ru);
   await expect(page.locator(".viewer-area")).not.toHaveAttribute("data-displayed-revision",previous);
   await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
   const moved=await projectById(current.id);expect(moved.revisions.some(r=>r.id===committed.currentRevision)).toBe(true);
   const metrics=await invoke("inspect_model",{projectId:moved.id,revisionId:moved.currentRevision});expect(metrics.volumeMm3).toBeCloseTo(volume,3);item.history[direction]=metrics;
  }
  const final=await projectById(current.id);await liveThemes(final,locale,theme);item.liveThemeSwitchKeptScene=true;
  await home();
  await expect(page.locator(".project-card").filter({has:page.locator(".project-card-info > button").filter({hasText:final.name})}).locator(".project-art img")).toBeVisible();
  const homeProjects=await invoke("list_projects");
  await fileInput.setInputFiles(filename);
  let create=page.getByRole("dialog",{name:tr(locale,"Create a project from selected files"),exact:true});
  await expect(create).toBeVisible();await create.getByRole("button",{name:tr(locale,"Cancel"),exact:true}).click();
  expect(await invoke("list_projects")).toEqual(homeProjects);
  await fileInput.setInputFiles(filename);create=page.getByRole("dialog",{name:tr(locale,"Create a project from selected files"),exact:true});
  const newName=`IGES Home ${locale} ${theme} ${randomUUID().slice(0,8)}`;
  await create.getByRole("textbox",{name:tr(locale,"Project name"),exact:true}).fill(newName);
  await create.getByRole("button",{name:tr(locale,"Create project"),exact:true}).click();
  dialog=await previewDialog(locale);
  expect(await dialog.getByRole("checkbox").count()).toBe(0);
  const empty=(await invoke("list_projects")).find(project=>project.name===newName);
  expect(empty.currentRevision).toBeNull();expect(empty.revisions).toHaveLength(0);expect(empty.files).toHaveLength(0);
  await dialog.getByRole("button",{name:tr(locale,"Import IGES"),exact:true}).click();await approve(ru);
  await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
  const first=await projectById(empty.id);expect(first.revisions).toHaveLength(1);
  item.homeImport=await invoke("inspect_model",{projectId:first.id,revisionId:first.currentRevision});
  expect(item.homeImport.volumeMm3).toBeCloseTo(expectedVolume,3);expect(item.homeImport.areaMm2).toBeCloseTo(expectedArea,3);
  await home();
 }
 if(report.errors.length)throw Error("Unhandled JavaScript errors in native WebView");report.status="PASS";
}catch(error){report.status="FAIL";report.failure=error.stack??String(error);if(page)await page.screenshot({path:path.join(runtime,"iges-failure.png")}).catch(()=>{});process.exitCode=1;}
finally{if(saveChild&&saveChild.exitCode===null)saveChild.kill();fs.writeFileSync(output,JSON.stringify(report,null,2));console.log(JSON.stringify({status:report.status,cases:report.cases.length,failure:report.failure??null}));if(browser)await browser.close();}
