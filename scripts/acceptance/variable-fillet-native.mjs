/** Real built-app UI → IPC → host → OCCT acceptance. Isolated profile required. No IPC mocks or failed-call retries. */
import fs from "node:fs";
import path from "node:path";
import {randomUUID} from "node:crypto";
import {chromium,expect as baseExpect} from "@playwright/test";
import * as THREE from "three";
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
async function home(){if(await page.locator(".viewer-area").count())await page.locator("button.brand").click();await expect(page.locator(".dashboard")).toBeVisible();}
async function seed(name){
 const now=new Date().toISOString(),project={schemaVersion:1,id:randomUUID(),name:name+" "+randomUUID().slice(0,8),units:"mm",agent:"codex",pinned:false,createdAt:now,updatedAt:now,revisions:[],currentRevision:null,messages:[],files:[],exports:[]};
 await invoke("save_project",{project});
 const id=await invoke("request_permission",{projectId:project.id,action:"modify_project",detail:"Isolated variable-fillet acceptance fixture"});
 await invoke("resolve_permission",{id,allow:true});
 const literal=mm=>({kind:"literal",mm});
 const document={schemaVersion:2,revisionId:"candidate",parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle",width:literal(40),depth:literal(20)}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:literal(10)}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]};
 const created=await invoke("apply_program",{projectId:project.id,program:JSON.stringify(document),prompt:"Variable-fillet acceptance baseline",expectedRevision:null});
 await expect(page.locator(".project-card").filter({has:page.locator(".project-card-info > button").filter({hasText:created.name})}).locator(".project-art img")).toBeVisible();
 return created;
}
async function openProject(project){
 await page.reload();await expect(page.locator(".dashboard")).toBeVisible();
 await page.locator(".project-card-info > button").filter({hasText:project.name}).click();
 await expect(page.locator(".viewer-area")).toHaveAttribute("data-displayed-revision",project.currentRevision);
 await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
}
async function openVariable(project,ru){
 await page.getByRole("button",{name:ru?"Спереди":"Front",exact:true}).click();
 await settledCamera(project.id);
 await page.getByRole("button",{name:ru?"Выбрать CAD-ребро":"Select CAD edge",exact:true}).click();
 const canvas=page.locator(".viewer-area canvas"),bounds=await canvas.boundingBox();
 const saved=JSON.parse(await page.evaluate(key=>localStorage.getItem(key),cameraKey(project.id)));
 const camera=new THREE.PerspectiveCamera(38,bounds.width/bounds.height,.1,10000);
 camera.position.fromArray(saved.position);camera.quaternion.fromArray(saved.quaternion);camera.up.fromArray(saved.up);camera.zoom=saved.zoom;camera.updateProjectionMatrix();camera.updateMatrixWorld(true);
 // Real source BREP: CAD (20,-10,5) → render (20,5,10), mm.
 const point=new THREE.Vector3(20,5,10).project(camera);
 const position={x:(point.x+1)*bounds.width/2,y:(1-point.y)*bounds.height/2};
 await canvas.hover({position});await canvas.click({position});
 await page.locator('[data-editor="viewport3d"]').focus();await page.keyboard.press("Control+b");
 const blend=page.getByRole("dialog",{name:ru?"Обработать выбранные рёбра":"Blend selected edges",exact:true});
 await expect(blend).toBeVisible();
 await blend.getByLabel(ru?"Обработка ребра":"Edge treatment",{exact:true}).selectOption("variable");
 const dialog=page.getByRole("dialog",{name:ru?"Скругление переменного радиуса":"Variable radius fillet",exact:true});
 await expect(dialog.locator("output")).toHaveCount(2);
 return dialog;
}
async function approve(ru){await page.getByRole("button",{name:ru?"Разрешить один раз":"Allow once",exact:true}).click();}
try{
 browser=await chromium.connectOverCDP(endpoint);
 page=browser.contexts().flatMap(context=>context.pages()).find(page=>/tauri\.localhost/.test(page.url()));
 if(!page)throw Error("Isolated Tauri page missing");
 page.setDefaultTimeout(30000);page.on("pageerror",error=>report.errors.push({message:error.message,stack:error.stack}));
 if(await page.getByRole("dialog").count())await page.getByRole("dialog").locator(".modal-close").click();
 await home();
 for(const locale of ["ru","en"])for(const theme of ["dark","light"]){
  const ru=locale==="ru",fractional=locale==="en"&&theme==="light",r1=fractional?1.25:1,r2=fractional?3.5:3;
  const item={locale,theme,radiusStart:r1,radiusEnd:r2};report.cases.push(item);
  await home();const preferences=await invoke("get_ui_preferences");
  await invoke("set_ui_preferences",{value:{...preferences,"forma.locale":locale,"forma.theme":theme}});
  await page.evaluate(({locale,theme})=>{localStorage.setItem("forma.locale",locale);localStorage.setItem("forma.theme",theme);},{locale,theme});
  await page.reload();await expect(page.locator("html")).toHaveAttribute("lang",locale);await expect(page.locator("html")).toHaveAttribute("data-theme",theme);
  const baseline=await seed("M4 variable-fillet "+locale+" "+theme);
  await openProject(baseline);let dialog=await openVariable(baseline,ru);
  const start=()=>dialog.getByRole("textbox",{name:ru?"Радиус в начале (мм)":"Start radius (mm)",exact:true});
  const end=()=>dialog.getByRole("textbox",{name:ru?"Радиус в конце (мм)":"End radius (mm)",exact:true});
  const build=()=>dialog.getByRole("button",{name:ru?"Построить":"Build",exact:true});
  item.endpoints=await dialog.locator("output").allTextContents();
  await start().fill("");await expect(build()).toBeDisabled();
  await start().fill("0");await expect(build()).toBeDisabled();
  await start().fill("50");await end().fill("50");
  await expect(dialog.getByText(ru?"Не удалось построить переменное скругление. Уменьшите радиусы и проверьте выбранное ребро.":"Could not build the variable fillet. Reduce the radii and check the selected edge.",{exact:false})).toBeVisible();
  await expect(build()).toBeDisabled();
  await dialog.getByRole("button",{name:ru?"Отмена":"Cancel",exact:true}).click();
  expect((await projectById(baseline.id)).currentRevision).toBe(baseline.currentRevision);
  dialog=await openVariable(baseline,ru);
  await start().fill(String(r1));await end().fill(String(r2));
  await dialog.getByRole("button",{name:ru?"Поменять радиусы местами":"Swap endpoint radii",exact:true}).click();
  await expect(start()).toHaveValue(String(r2));await expect(end()).toHaveValue(String(r1));
  await dialog.getByRole("button",{name:ru?"Поменять радиусы местами":"Swap endpoint radii",exact:true}).click();
  await expect(dialog.getByText(ru?"Геометрия черновика корректна":"Draft geometry is valid",{exact:false})).toBeVisible();
  const canvas=dialog.locator(".model-preview-canvas canvas");await canvas.hover();await page.bringToFront();
  await expect.poll(async()=>foregroundPixels(await canvas.screenshot()),{message:"Real variable fillet preview contains geometry"}).toBeGreaterThan(200);
  item.previewForeground=foregroundPixels(await canvas.screenshot());
  await dialog.screenshot({path:path.join(path.dirname(output),`variable-fillet-${locale}-${theme}.png`)});
  await build().click();await approve(ru);
  await expect(page.locator(".viewer-area")).not.toHaveAttribute("data-displayed-revision",baseline.currentRevision);
  await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
  const committed=await projectById(baseline.id),doc=JSON.parse(committed.revisions.find(rev=>rev.id===committed.currentRevision).program),old=JSON.parse(baseline.revisions.at(-1).program);
  expect(committed.revisions.length).toBe(baseline.revisions.length+1);expect(doc.features.slice(0,old.features.length)).toEqual(old.features);
  expect(committed.revisions.find(rev=>rev.id===baseline.currentRevision).program).toBe(baseline.revisions.at(-1).program);
  expect(doc.features.at(-1).operation).toMatchObject({type:"variableFillet",radiusStart:{mm:r1},radiusEnd:{mm:r2},reference:{role:"box-edge:z:xmax:ymin"}});
  item.properties=await invoke("inspect_model",{projectId:committed.id,revisionId:committed.currentRevision});
  expect(item.properties.volumeMm3).toBeCloseTo(fractional?7986.76592388861:7990.52372746023,3);
  expect(item.properties.areaMm2).toBeCloseTo(fractional?2786.9327101418:2789.33014059995,3);
  expect(item.properties.faceCount).toBe(7);expect(item.properties.edgeCount).toBe(15);
  item.revisionsAdded=1;item.history={};
  for(const [direction,index,volume] of [["undo",0,8000],["redo",1,item.properties.volumeMm3]]){
   const previous=(await projectById(baseline.id)).currentRevision;
   await page.locator(".model-history-controls button").nth(index).click();await approve(ru);
   await expect(page.locator(".viewer-area")).not.toHaveAttribute("data-displayed-revision",previous);
   await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
   const moved=await projectById(baseline.id);expect(moved.revisions.some(rev=>rev.id===committed.currentRevision)).toBe(true);
   const metrics=await invoke("inspect_model",{projectId:moved.id,revisionId:moved.currentRevision});expect(metrics.volumeMm3).toBeCloseTo(volume,5);
   item.history[direction]=metrics;
  }
  const liveCanvas=await page.locator(".viewer-area canvas").elementHandle();
  const liveRevision=(await projectById(baseline.id)).currentRevision;
  await settledCamera(baseline.id);
  const liveCamera=await page.evaluate(key=>localStorage.getItem(key),cameraKey(baseline.id));
  await page.getByRole("button",{name:ru?"Настройки":"Settings",exact:true}).first().click();
  await page.getByRole("button",{name:ru?"Внешний вид":"Appearance",exact:true}).click();
  const settings=page.getByRole("dialog",{name:ru?"Настройки":"Settings",exact:true});
  for(const next of [theme==="dark"?"light":"dark",theme]){
   await settings.getByRole("combobox").nth(1).click();
   await page.getByRole("option",{name:next==="dark"?(ru?"Тёмная":"Dark"):(ru?"Светлая":"Light"),exact:true}).click();
   await expect(settings.getByRole("combobox").nth(1)).toHaveAttribute("data-state","closed");
   await expect(page.locator("html")).toHaveAttribute("data-theme",next);
   expect(await liveCanvas.evaluate(element=>element.isConnected)).toBe(true);
   await expect(page.locator(".viewer-area")).toHaveAttribute("data-displayed-revision",liveRevision);
   expect(await page.evaluate(key=>localStorage.getItem(key),cameraKey(baseline.id))).toBe(liveCamera);
  }
  await settings.locator(".modal-close").click();await expect(settings).toHaveCount(0);
  item.liveThemeSwitchKeptCanvasRevisionAndCamera=true;
  await home();
 }
 if(report.errors.length)throw Error("JavaScript errors in native WebView");report.status="PASS";
}catch(error){report.status="FAIL";report.failure=error.stack??String(error);if(page)await page.screenshot({path:path.join(path.dirname(output),"variable-fillet-failure.png")}).catch(()=>{});process.exitCode=1;}
finally{fs.writeFileSync(output,JSON.stringify(report,null,2));console.log(JSON.stringify({status:report.status,cases:report.cases.length,failure:report.failure??null}));if(browser)await browser.close();}
