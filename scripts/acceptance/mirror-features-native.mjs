/**
 * Built-app MirrorFeatures acceptance. Requires the same isolated Windows host,
 * real OCCT worker and loopback CDP setup documented in docs/en/mirror-features.md.
 * Usage: FORMA_NATIVE_ACCEPTANCE=isolated node scripts/acceptance/mirror-features-native.mjs http://127.0.0.1:9336 <report.json>
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
async function home(){if(await page.locator(".viewer-area").count())await page.locator("button.brand").click();await expect(page.locator(".dashboard")).toBeVisible();}
async function seed(name,document){
 const now=new Date().toISOString(),project={schemaVersion:1,id:randomUUID(),name:name+" "+randomUUID().slice(0,8),units:"mm",agent:"codex",pinned:false,createdAt:now,updatedAt:now,revisions:[],currentRevision:null,messages:[],files:[],exports:[]};
 await invoke("save_project",{project});
 const id=await invoke("request_permission",{projectId:project.id,action:"modify_project",detail:"Isolated thicken acceptance fixture"});
 await invoke("resolve_permission",{id,allow:true});
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

const literal=mm=>({kind:"literal",mm});
function fixture(mode){
 const doc={schemaVersion:2,revisionId:"candidate",parameters:[],features:[
  {id:"profile",name:"Profile",operation:{type:"rectangle",width:literal(40),depth:literal(20)}},
  {id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:literal(10)}}
 ],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]};
 if(mode==="sphere")return {doc:{...doc,features:[{id:"ball",name:"Sphere",operation:{type:"sphere",radius:literal(10)}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"ball"}]},first:"ball",ids:["ball"],base:null};
 if(mode!=="holes"){
  doc.features.push({id:"circle",name:"Boss profile",operation:{type:"circle",radius:literal(2)}},{id:"boss",name:"Boss",operation:{type:"extrude",sketchId:"circle",distance:literal(4)}},{id:"placed",name:"Placed boss",operation:{type:"translate",bodyFeatureId:"boss",offsetMm:[10,0,10]}},{id:"withBoss",name:"Add boss",operation:{type:"boolean",leftFeatureId:"pad",rightFeatureId:"placed",mode:"union"}});
  doc.bodies[0].sourceFeatureId="withBoss";
 }
 if(mode!=="boss"){
  const radius=mode==="holes"?2:1,x=mode==="holes"?10:5;
  doc.features.push({id:"holeCircle",name:"Hole profile",operation:{type:"circle",radius:literal(radius)}},{id:"cutter",name:"Cutter",operation:{type:"extrude",sketchId:"holeCircle",distance:literal(20)}},{id:"holePlaced",name:"Placed cutter",operation:{type:"translate",bodyFeatureId:"cutter",offsetMm:[x,0,-5]}},{id:"withHole",name:"Cut hole",operation:{type:"boolean",leftFeatureId:doc.bodies[0].sourceFeatureId,rightFeatureId:"holePlaced",mode:"cut"}});
  doc.bodies[0].sourceFeatureId="withHole";
 }
 return {doc,first:mode==="holes"?"withHole":"withBoss",ids:mode==="combined"?["withBoss","withHole"]:[mode==="holes"?"withHole":"withBoss"],base:"pad"};
}
async function choose(dialog,index,value){
 await dialog.getByRole("combobox").nth(index).click();
 await page.locator('.select-menu[data-state="open"] .select-option[data-value="'+value+'"]').click();
}
async function openMirror(ru,first){
 await page.getByRole("menuitem",{name:ru?"Инструменты":"Tools",exact:true}).click();
 await page.getByRole("menuitem",{name:ru?"Зеркалировать операции":"Mirror features",exact:true}).click();
 const dialog=page.getByRole("dialog",{name:ru?"Зеркалировать операции":"Mirror features",exact:true});
 await choose(dialog,1,first);return dialog;
}
try{
 browser=await chromium.connectOverCDP(endpoint);
 page=browser.contexts().flatMap(c=>c.pages()).find(p=>/tauri\.localhost/.test(p.url()));
 if(!page)throw Error("Isolated Tauri page missing");
 page.setDefaultTimeout(30000);page.on("pageerror",error=>report.errors.push({message:error.message,stack:error.stack}));
 await expect(page.locator(".dashboard")).toBeVisible();
 for(const [locale,theme,mode] of [["ru","dark","boss"],["ru","light","combined"],["en","dark","holes"],["en","light","sphere"]]){
  const ru=locale==="ru",item={locale,theme,mode};report.cases.push(item);
  await home();const prefs=await invoke("get_ui_preferences");
  await invoke("set_ui_preferences",{value:{...prefs,"forma.locale":locale,"forma.theme":theme}});
  await page.evaluate(({locale,theme})=>{localStorage.setItem("forma.locale",locale);localStorage.setItem("forma.theme",theme);},{locale,theme});
  await page.reload();await expect(page.locator("html")).toHaveAttribute("lang",locale);await expect(page.locator("html")).toHaveAttribute("data-theme",theme);
  const model=fixture(mode),baseline=await seed("M4 mirror "+locale+" "+theme,model.doc);
  item.before=await invoke("inspect_model",{projectId:baseline.id,revisionId:baseline.currentRevision});
  await openProject(baseline);await page.locator(".viewer-area canvas").hover();
  let dialog=await openMirror(ru,model.first);
  const offsetLabel=ru?"Смещение плоскости (мм)":"Plane offset (mm)",build=ru?"Построить":"Build";
  await choose(dialog,2,"y");
  await expect(dialog.getByText(ru?"Зеркальные изменения не меняют это тело.":"The mirrored changes do not alter this body.",{exact:true})).toBeVisible();
  await expect(dialog.getByRole("button",{name:build,exact:true})).toBeDisabled();
  expect((await projectById(baseline.id)).currentRevision).toBe(baseline.currentRevision);
  await choose(dialog,2,"x");
  if(mode==="boss"||mode==="sphere"){
   await dialog.getByLabel(offsetLabel,{exact:true}).fill("100");
   await expect(dialog.getByText(ru?"После зеркалирования должен остаться один связный солид.":"Mirrored changes must leave one connected solid.",{exact:true})).toBeVisible();
   await expect(dialog.getByRole("button",{name:build,exact:true})).toBeDisabled();
  }
  await dialog.getByRole("button",{name:ru?"Отмена":"Cancel",exact:true}).click();await expect(dialog).toBeHidden();expect((await projectById(baseline.id)).currentRevision).toBe(baseline.currentRevision);item.cancelDidNotCommit=true;
  dialog=await openMirror(ru,model.first);
  const offset=mode==="sphere"?5.05:0;
  await dialog.getByLabel(offsetLabel,{exact:true}).fill(String(offset));
  await expect(dialog.getByText(ru?"Геометрия черновика корректна":"Draft geometry is valid",{exact:true})).toBeVisible();
  await expect(dialog.getByRole("button",{name:build,exact:true})).toBeEnabled();
  const canvas=dialog.locator(".model-preview-canvas canvas");await canvas.hover();await page.bringToFront();
  await expect.poll(async()=>foregroundPixels(await canvas.screenshot())).toBeGreaterThan(200);
  await page.screenshot({path:path.join(path.dirname(output),"mirror-"+locale+"-"+theme+".png")});
  await dialog.getByRole("button",{name:build,exact:true}).click();await page.getByRole("button",{name:ru?"Разрешить один раз":"Allow once",exact:true}).click();
  await expect(page.locator(".viewer-area")).not.toHaveAttribute("data-displayed-revision",baseline.currentRevision);
  await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
  const committed=await projectById(baseline.id),doc=JSON.parse(committed.revisions.find(r=>r.id===committed.currentRevision).program),original=JSON.parse(baseline.revisions.find(r=>r.id===baseline.currentRevision).program);
  expect(committed.revisions.length).toBe(baseline.revisions.length+1);
  expect(doc.features.slice(0,original.features.length)).toEqual(original.features);
  expect(doc.features.at(-1).operation.featureIds).toEqual(model.ids);expect(doc.features.at(-1).operation.baseFeatureId).toBe(model.base);
  item.revisionsAdded=1;item.offsetMm=offset;item.totals=await invoke("inspect_model",{projectId:committed.id,revisionId:committed.currentRevision});
  const pi=Math.PI,d=offset*2;
  const expected=mode==="boss"?{volume:8000+32*pi,area:2800+32*pi,faces:10}:mode==="holes"?{volume:8000-80*pi,area:2800+64*pi,faces:8}:mode==="combined"?{volume:8000+12*pi,area:2800+68*pi,faces:12}:{volume:8000*pi/3-pi*(40+d)*(20-d)**2/12,area:400*pi+20*pi*d};
  expect(item.totals.volumeMm3).toBeCloseTo(expected.volume,3);expect(item.totals.areaMm2).toBeCloseTo(expected.area,3);if(expected.faces)expect(item.totals.faceCount).toBe(expected.faces);
  item.history={};
  for(const [direction,index,volume] of [["undo",0,item.before.volumeMm3],["redo",1,expected.volume]]){
   const previous=(await projectById(baseline.id)).currentRevision;
   await page.locator(".model-history-controls button").nth(index).click();
   await page.getByRole("button",{name:ru?"Разрешить один раз":"Allow once",exact:true}).click();
   await expect(page.locator(".viewer-area")).not.toHaveAttribute("data-displayed-revision",previous);await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
   const moved=await projectById(baseline.id);expect(moved.revisions.some(r=>r.id===committed.currentRevision)).toBe(true);
   const properties=await invoke("inspect_model",{projectId:moved.id,revisionId:moved.currentRevision});expect(properties.volumeMm3).toBeCloseTo(volume,3);item.history[direction]={properties,mirrorRevisionRetained:true};
  }
  await home();
 }
 if(report.errors.length)throw Error("Native WebView JavaScript errors");report.status="PASS";
}catch(error){report.status="FAIL";report.failure=error.stack??String(error);if(page)await page.screenshot({path:path.join(path.dirname(output),"mirror-failure.png")}).catch(()=>{});process.exitCode=1;}
finally{fs.writeFileSync(output,JSON.stringify(report,null,2));console.log(JSON.stringify({status:report.status,cases:report.cases.length,failure:report.failure??null}));if(browser)await browser.close();}
