/**
 * Built-app ThickenFace acceptance. Requires the same isolated Windows host,
 * real OCCT worker and loopback CDP setup documented in docs/en/thicken.md.
 * Usage: FORMA_NATIVE_ACCEPTANCE=isolated node scripts/acceptance/thicken-native.mjs http://127.0.0.1:9336 <report.json>
 * IPC is never replaced. Fixtures wait for visible thumbnail publication;
 * failed IPC calls are not retried.
 */
import fs from "node:fs";
import path from "node:path";
import {randomUUID} from "node:crypto";
import {chromium,expect as baseExpect} from "@playwright/test";
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
 const id=await invoke("request_permission",{projectId:project.id,action:"modify_project",detail:"Isolated thicken acceptance fixture"});
 await invoke("resolve_permission",{id,allow:true});
 const literal=mm=>({kind:"literal",mm});
 const document={schemaVersion:2,revisionId:"candidate",parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle",width:literal(40),depth:literal(20)}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:literal(10)}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]};
 const created=await invoke("apply_program",{projectId:project.id,program:JSON.stringify(document),prompt:"Thicken acceptance baseline",expectedRevision:null});
 await expect(page.locator(".project-card").filter({has:page.locator(".project-card-info > button").filter({hasText:created.name})}).locator(".project-art img")).toBeVisible();
 return created;
}
async function openProject(project){
 await page.reload();await expect(page.locator(".dashboard")).toBeVisible();
 await page.locator(".project-card-info > button").filter({hasText:project.name}).click();
 await expect(page.locator(".viewer-area")).toHaveAttribute("data-displayed-revision",project.currentRevision);
 await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
}
async function pickUpperFace(project,ru){
 await page.getByRole("button",{name:ru?"Сверху":"Top",exact:true}).click();
 await settledCamera(project.id);
 await page.getByRole("button",{name:ru?"Выбрать CAD-грань":"Select CAD face",exact:true}).click();
 const canvas=page.locator(".viewer-area canvas"),bounds=await canvas.boundingBox();
 // Stay on the selected planar face, clear of the existing central extrusion gizmo.
 const position={x:bounds.width*.6,y:bounds.height*.55};
 await canvas.hover({position});await canvas.click({position});
}
async function openThicken(ru){
 await page.getByRole("menuitem",{name:ru?"Инструменты":"Tools",exact:true}).click();
 const title=ru?"Утолщить грань":"Thicken face";
 const action=page.getByRole("menuitem",{name:title,exact:true});await expect(action).toBeEnabled();await action.click();
 const dialog=page.getByRole("dialog",{name:title,exact:true});
 await expect(dialog.getByText("body · box-face:zmax",{exact:false})).toBeVisible();
 return dialog;
}
async function approve(ru){await page.getByRole("button",{name:ru?"Разрешить один раз":"Allow once",exact:true}).click();}
try{
 browser=await chromium.connectOverCDP(endpoint);
 page=browser.contexts().flatMap(context=>context.pages()).find(page=>/tauri\.localhost/.test(page.url()));
 if(!page)throw Error("Isolated Tauri page missing");
 page.setDefaultTimeout(30000);
 page.on("pageerror",error=>report.errors.push({message:error.message,stack:error.stack}));
 await expect(page.locator(".dashboard")).toBeVisible();
 for(const locale of ["ru","en"])for(const theme of ["dark","light"]){
  const ru=locale==="ru",offset=locale==="en"&&theme==="light"?-2:2,item={locale,theme,offsetMm:offset};report.cases.push(item);
  await home();
  const preferences=await invoke("get_ui_preferences");
  await invoke("set_ui_preferences",{value:{...preferences,"forma.locale":locale,"forma.theme":theme}});
  await page.evaluate(({locale,theme})=>{localStorage.setItem("forma.locale",locale);localStorage.setItem("forma.theme",theme);},{locale,theme});
  await page.reload();await expect(page.locator("html")).toHaveAttribute("lang",locale);await expect(page.locator("html")).toHaveAttribute("data-theme",theme);
  const baseline=await seed("M4 thicken "+locale+" "+theme);
  item.before=await invoke("inspect_model",{projectId:baseline.id,revisionId:baseline.currentRevision});
  expect(item.before.volumeMm3).toBeCloseTo(8000,5);
  await openProject(baseline);await pickUpperFace(baseline,ru);
  let dialog=await openThicken(ru);
  const label=ru?"Толщина со знаком (мм)":"Signed thickness (mm)",build=ru?"Построить":"Build";
  await dialog.getByLabel(label,{exact:true}).fill("0");await expect(dialog.getByRole("button",{name:build,exact:true})).toBeDisabled();
  await dialog.getByLabel(label,{exact:true}).fill("");await expect(dialog.getByRole("button",{name:build,exact:true})).toBeDisabled();
  await page.keyboard.press("Escape");
  expect((await projectById(baseline.id)).currentRevision).toBe(baseline.currentRevision);
  item.cancelDidNotCommit=true;
  await pickUpperFace(baseline,ru);dialog=await openThicken(ru);
  await dialog.getByLabel(label,{exact:true}).fill(String(offset));
  await expect(dialog.getByText(ru?"Геометрия черновика корректна":"Draft geometry is valid",{exact:true})).toBeVisible();
  await expect(dialog.getByRole("button",{name:build,exact:true})).toBeEnabled();
  const preview=dialog.locator(".model-preview-canvas canvas");await preview.hover();await page.bringToFront();
  await expect.poll(async()=>foregroundPixels(await preview.screenshot()),{message:"Real thicken preview contains rendered geometry"}).toBeGreaterThan(200);
  await page.screenshot({path:path.join(path.dirname(output),"thicken-"+locale+"-"+theme+".png")});
  await dialog.getByRole("button",{name:build,exact:true}).click();await approve(ru);
  await expect(page.locator(".viewer-area")).not.toHaveAttribute("data-displayed-revision",baseline.currentRevision);
  await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
  const committed=await projectById(baseline.id),doc=JSON.parse(committed.revisions.find(rev=>rev.id===committed.currentRevision).program);
  expect(committed.revisions.length).toBe(baseline.revisions.length+1);expect(doc.bodies.length).toBe(2);
  expect(doc.bodies.find(body=>body.id==="body").sourceFeatureId).toBe("pad");
  const plate=doc.bodies.find(body=>body.id!=="body");
  item.plate=await invoke("measure_model_reference",{projectId:committed.id,expectedRevision:committed.currentRevision,bodyId:plate.id,query:{kind:"bodyMetrics"}});
  item.original=await invoke("measure_model_reference",{projectId:committed.id,expectedRevision:committed.currentRevision,bodyId:"body",query:{kind:"bodyMetrics"}});
  item.totals=await invoke("inspect_model",{projectId:committed.id,revisionId:committed.currentRevision});
  expect(item.plate.result.volumeMm3).toBeCloseTo(1600,5);expect(item.plate.result.areaMm2).toBeCloseTo(1840,5);
  expect(item.plate.result.faceCount).toBe(6);expect(item.original.result.volumeMm3).toBeCloseTo(8000,5);
  expect(item.totals.volumeMm3).toBeCloseTo(9600,5);expect(item.totals.areaMm2).toBeCloseTo(4640,5);expect(item.totals.faceCount).toBe(12);
  item.revisionsAdded=1;item.history={};
  for(const [direction,index,bodyCount,volume] of [["undo",0,1,8000],["redo",1,2,9600]]){
   const previous=(await projectById(baseline.id)).currentRevision;
   await page.locator(".model-history-controls button").nth(index).click();await approve(ru);
   await expect(page.locator(".viewer-area")).not.toHaveAttribute("data-displayed-revision",previous);
   await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
   const moved=await projectById(baseline.id),document=JSON.parse(moved.revisions.find(rev=>rev.id===moved.currentRevision).program);
   expect(document.bodies.length).toBe(bodyCount);expect(moved.revisions.some(rev=>rev.id===committed.currentRevision)).toBe(true);
   const properties=await invoke("inspect_model",{projectId:moved.id,revisionId:moved.currentRevision});expect(properties.volumeMm3).toBeCloseTo(volume,5);
   item.history[direction]={bodyCount,properties,savedThickenRevisionRetained:true};
  }
  await home();
 }
 if(report.errors.length)throw Error("JavaScript errors in native WebView");
 report.status="PASS";
}catch(error){report.status="FAIL";report.failure=error.stack??String(error);if(page)await page.screenshot({path:path.join(path.dirname(output),"thicken-failure.png")}).catch(()=>{});process.exitCode=1;}
finally{fs.writeFileSync(output,JSON.stringify(report,null,2));console.log(JSON.stringify({status:report.status,cases:report.cases.length,failure:report.failure??null}));if(browser)await browser.close();}
