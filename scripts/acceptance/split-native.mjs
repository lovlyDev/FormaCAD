/**
 * Real Windows WebView acceptance: UI -> Tauri IPC -> host -> OCCT worker.
 * Run only against an isolated FORMA_DATA_DIR and WEBVIEW2_USER_DATA_FOLDER.
 * Start the embedded-frontend app with loopback CDP port 9336, then run:
 * FORMA_NATIVE_ACCEPTANCE=isolated node scripts/acceptance/split-native.mjs http://127.0.0.1:9336 <report.json>
 * Creates fixture projects and changes locale/theme in that isolated profile.
 * Does not replace invoke, install IPC mocks, build installers or publish artifacts.
 */
import fs from "node:fs";
import { foregroundPixels } from "./canvasPixels.mjs";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { chromium,expect as baseExpect } from "@playwright/test";
const expect=baseExpect.configure({timeout:30000});
const endpoint=process.argv[2],output=process.argv[3];
if(process.env.FORMA_NATIVE_ACCEPTANCE!=="isolated"||endpoint!=="http://127.0.0.1:9336"||!output)throw Error("Use the owned M4 loopback endpoint and output path");
const report={ipcMocked:false,setup:"real native commands with one-use grants; baseline waits for visible dashboard thumbnail; no failed IPC retries",cases:[],errors:[]};
let browser,page;
const invoke=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const literal=mm=>({kind:"literal",mm});
const box=()=>({schemaVersion:2,revisionId:"candidate",parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle",width:literal(40),depth:literal(20)}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:literal(10)}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]});
async function seed(name,document){ name += " "+randomUUID().slice(0,8);
 const now=new Date().toISOString(),project={schemaVersion:1,id:randomUUID(),name,units:"mm",agent:"codex",pinned:false,createdAt:now,updatedAt:now,revisions:[],currentRevision:null,messages:[],files:[],exports:[]};
 await invoke("save_project",{project});
 const id=await invoke("request_permission",{projectId:project.id,action:"modify_project",detail:"Owned M4 acceptance baseline"});
 await invoke("resolve_permission",{id,allow:true});
 const created=await invoke("apply_program",{projectId:project.id,program:JSON.stringify(document),prompt:"M4 acceptance baseline",expectedRevision:null});
 // Wait for the visible result of the background metadata write, without retrying any failed IPC.
 await expect(page.locator(".project-card").filter({has:page.locator(".project-card-info > button").filter({hasText:created.name})}).locator(".project-art img")).toBeVisible();
 return created;
}
async function home(){if(await page.locator(".viewer-area").count())await page.locator("button.brand").click();await expect(page.locator(".dashboard")).toBeVisible();}
async function openProject(project){
 await page.reload();await expect(page.locator(".dashboard")).toBeVisible();
 await page.locator(".project-card-info > button").filter({hasText:project.name}).click();
 await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
 await expect(page.locator(".viewer-area")).toHaveAttribute("data-displayed-revision",project.currentRevision);
  const canvas=page.locator(".viewer-area canvas");
  await canvas.hover();
  const key=`forma.ui.project.${project.id}.view.main.camera`;
  const camera=()=>page.evaluate(key=>JSON.parse(localStorage.getItem(key)??"null"),key);
  await expect.poll(async()=>(await camera())?.position?.length??0).toBe(3);
  let priorCamera="",stable=0;
  await expect.poll(async()=>{
   const current=JSON.stringify(await camera());
   stable=current===priorCamera?stable+1:0;priorCamera=current;return stable;
  },{message:"Camera settles before testing wheel zoom"}).toBeGreaterThan(2);
  const beforeCamera=await camera();
  await page.mouse.wheel(0,-180);
  const distance=current=>Math.hypot(...current.position.map((v,i)=>v-current.target[i]));
  await expect.poll(async()=>distance(beforeCamera)-distance(await camera()),{message:"Wheel zooms the real camera toward its target"}).toBeGreaterThan(.01);
  report.inputChecks??=[];
  report.inputChecks.push({projectId:project.id,hover:true,wheelChangesCamera:true});
}
async function projectById(id){return (await invoke("list_projects")).find(p=>p.id===id);}
async function menu(label,ru){await page.getByRole("menuitem",{name:ru?"Инструменты":"Tools",exact:true}).click();await page.getByRole("menuitem",{name:label,exact:true}).click();}
try{
 browser=await chromium.connectOverCDP(endpoint);
 page=browser.contexts().flatMap(c=>c.pages()).find(p=>/tauri\.localhost/.test(p.url()));
 if(!page)throw Error("Owned Tauri page is missing");
 page.setDefaultTimeout(30000);
 page.on("pageerror",error=>report.errors.push({message:error.message,stack:error.stack,locale:report.cases.at(-1)?.locale,theme:report.cases.at(-1)?.theme}));
 await expect(page.locator(".dashboard")).toBeVisible();
 for(const locale of ["ru","en"])for(const theme of ["dark","light"]){
  const ru=locale==="ru",item={locale,theme,split:null,empty:null};report.cases.push(item);
  await home();
  const preferences=await invoke("get_ui_preferences");
  await invoke("set_ui_preferences",{value:{...preferences,"forma.locale":locale,"forma.theme":theme}});
  await page.evaluate(({locale,theme})=>{localStorage.setItem("forma.locale",locale);localStorage.setItem("forma.theme",theme);},{locale,theme});
  await page.reload();await expect(page.locator("html")).toHaveAttribute("lang",locale);await expect(page.locator("html")).toHaveAttribute("data-theme",theme);
  const baseline=await seed("M4 split "+locale+" "+theme,box());
  const before=await invoke("inspect_model",{projectId:baseline.id,revisionId:baseline.currentRevision});
  await openProject(baseline);
  const splitTitle=ru?"Разделить тело плоскостью":"Split body by plane";
  await menu(splitTitle,ru);
  const dialog=page.getByRole("dialog",{name:splitTitle,exact:true});
  const offset=dialog.getByLabel(ru?"Смещение плоскости (мм)":"Plane offset (mm)",{exact:true});
  await offset.fill("100");
  await expect(dialog.getByText(ru?"Плоскость не разделяет тело на две объёмные части.":"The plane does not divide the body into two solid parts.",{exact:true})).toBeVisible();
  await expect(dialog.getByRole("button",{name:ru?"Построить":"Build",exact:true})).toBeDisabled();
  expect((await projectById(baseline.id)).currentRevision).toBe(baseline.currentRevision);
  await offset.fill("");await expect(dialog.getByRole("button",{name:ru?"Построить":"Build",exact:true})).toBeDisabled();
  await offset.fill("5");
  await expect(dialog.getByText(ru?"Геометрия черновика корректна":"Draft geometry is valid",{exact:true})).toBeVisible();
  await expect(dialog.locator(".model-preview-canvas canvas")).toBeVisible();
  await dialog.locator(".model-preview-canvas canvas").hover();
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  await page.bringToFront();
  await expect.poll(async()=>foregroundPixels(await dialog.locator(".model-preview-canvas canvas").screenshot()),{timeout:30000,message:"Real WebGL preview must contain rendered geometry"}).toBeGreaterThan(200);
  await page.screenshot({path:path.join(path.dirname(output),"split-"+locale+"-"+theme+".png")});
  await dialog.getByRole("button",{name:ru?"Построить":"Build",exact:true}).click();
  await page.getByRole("button",{name:ru?"Разрешить один раз":"Allow once",exact:true}).click();
  await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
  await expect(page.locator(".viewer-area")).not.toHaveAttribute("data-displayed-revision",baseline.currentRevision);
  await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
  const committed=await projectById(baseline.id);
  expect(committed.revisions.length).toBe(baseline.revisions.length+1);
  const doc=JSON.parse(committed.revisions.find(r=>r.id===committed.currentRevision).program);
  expect(doc.bodies.length).toBe(2);
  const totals=await invoke("inspect_model",{projectId:committed.id,revisionId:committed.currentRevision});
  const halves=[];
  for(const body of doc.bodies)halves.push(await invoke("measure_model_reference",{projectId:committed.id,expectedRevision:committed.currentRevision,bodyId:body.id,query:{kind:"bodyMetrics"}}));
  item.split={before,totals,halves,revisionsAdded:committed.revisions.length-baseline.revisions.length};
  expect(totals.volumeMm3).toBeCloseTo(8000,5);expect(totals.areaMm2).toBeCloseTo(3200,5);expect(totals.faceCount).toBe(12);
  const volumes=halves.map(h=>h.result.volumeMm3).sort((a,b)=>a-b);expect(volumes[0]).toBeCloseTo(3000,5);expect(volumes[1]).toBeCloseTo(5000,5);
  const history={};
  for(const [direction,index,expectedBodies] of [["undo",0,1],["redo",1,2]]){
    const previous=(await projectById(baseline.id)).currentRevision;
    await expect(page.locator(".model-history-controls button").nth(index)).toBeEnabled();
    await page.locator(".model-history-controls button").nth(index).click();
    await page.getByRole("button",{name:ru?"Разрешить один раз":"Allow once",exact:true}).click();
    await expect(page.locator(".viewer-area")).not.toHaveAttribute("data-displayed-revision",previous);
    await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
    const moved=await projectById(baseline.id),program=JSON.parse(moved.revisions.find(r=>r.id===moved.currentRevision).program);
    expect(program.bodies.length).toBe(expectedBodies);
    expect(moved.revisions.some(r=>r.id===committed.currentRevision)).toBe(true);
    history[direction]={bodies:program.bodies.length,savedSplitRevisionRetained:true,properties:await invoke("inspect_model",{projectId:moved.id,revisionId:moved.currentRevision})};
  }
  item.split.history=history;
  const current=(await projectById(baseline.id)).currentRevision;
  await menu(splitTitle,ru);
  const rapid=page.getByRole("dialog",{name:splitTitle,exact:true});
  await rapid.getByRole("combobox").nth(0).click();
  await page.locator('.select-menu[data-state="open"] .select-option[data-value="body"]').click();
  const rapidOffset=rapid.getByLabel(ru?"Смещение плоскости (мм)":"Plane offset (mm)",{exact:true});
  for(let cycle=0;cycle<3;cycle++){
    await rapidOffset.fill("10");
    await expect(rapid.getByText(ru?"Геометрия черновика корректна":"Draft geometry is valid",{exact:true})).toBeVisible();
    await rapidOffset.fill("");
  }
  await page.keyboard.press("Escape");
  expect((await projectById(baseline.id)).currentRevision).toBe(current);
  item.split.rapidPreviewCycles=3;
  item.split.cancelDidNotCommit=true;
  await home();
  const pair=box();pair.features.push({id:"tool_profile",name:"Tool profile",operation:{type:"rectangle",width:literal(10),depth:literal(10)}},{id:"tool_pad",name:"Tool",operation:{type:"extrude",sketchId:"tool_profile",distance:literal(10)}},{id:"far",name:"Far",operation:{type:"translate",bodyFeatureId:"tool_pad",offsetMm:[100,0,0]}});pair.bodies.push({id:"tool",name:"Tool",sourceFeatureId:"far"});
  const booleanBase=await seed("M4 empty "+locale+" "+theme,pair);await openProject(booleanBase);
  const title=ru?"Булева операция":"Boolean operation";await menu(title,ru);
  const booleanDialog=page.getByRole("dialog",{name:title,exact:true});
  await booleanDialog.getByLabel(ru?"Режим булевой операции":"Boolean mode").selectOption("intersect");
  await expect(booleanDialog.getByText(ru?"Пересечение пустое.":"The intersection is empty.",{exact:true})).toBeVisible();
  await expect(booleanDialog.getByRole("button",{name:ru?"Построить":"Build",exact:true})).toBeDisabled();
  const unchanged=await projectById(booleanBase.id);expect(unchanged.currentRevision).toBe(booleanBase.currentRevision);
  item.empty={revisionUnchanged:true,properties:await invoke("inspect_model",{projectId:booleanBase.id,revisionId:booleanBase.currentRevision})};
  await page.screenshot({path:path.join(path.dirname(output),"empty-"+locale+"-"+theme+".png")});
  await page.keyboard.press("Escape");await home();
 }
 if(report.errors.length)throw Error("Native WebView reported JavaScript errors");
 report.status="PASS";
}catch(error){report.status="FAIL";report.failure=error.stack??String(error);if(page)await page.screenshot({path:path.join(path.dirname(output),"failure.png")}).catch(()=>{});process.exitCode=1;}
finally{fs.writeFileSync(output,JSON.stringify(report,null,2));console.log(JSON.stringify({status:report.status,cases:report.cases.length,failure:report.failure??null}));if(browser)await browser.close();}
