/**
 * Built-app OffsetProfile acceptance. Requires the same isolated Windows host,
 * real OCCT worker and loopback CDP setup documented in docs/en/offset-profile.md.
 * Usage: FORMA_NATIVE_ACCEPTANCE=isolated node scripts/acceptance/offset-profile-native.mjs http://127.0.0.1:9336 <report.json>
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
 const id=await invoke("request_permission",{projectId:project.id,action:"modify_project",detail:"Isolated offset acceptance fixture"});
 await invoke("resolve_permission",{id,allow:true});
 const created=await invoke("apply_program",{projectId:project.id,program:JSON.stringify(document),prompt:"Offset acceptance baseline",expectedRevision:null});
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
 let profile={type:"rectangle",width:literal(40),depth:literal(20)};
 if(mode==="circle")profile={type:"circle",radius:literal(10)};
 if(mode==="holes"){
  const coords=[[-20,-10],[20,-10],[20,10],[-20,10],[-5,-3],[-5,3],[5,3],[5,-3]];
  profile={type:"sketch2d",plane:"xy",originMm:[0,0,0],points:coords.map(([xMm,yMm],i)=>({id:"p"+i,xMm,yMm})),lines:coords.map((_,i)=>({id:"l"+i,startPointId:"p"+i,endPointId:"p"+(i<4?(i+1)%4:4+(i-3)%4),construction:false})),constraints:[]};
 }
 return {schemaVersion:2,revisionId:"candidate",parameters:[{id:"height",name:"Height",valueMm:10}],features:[
  {id:"profile",name:"Profile",operation:profile},
  {id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:{kind:"parameter",parameterId:"height"}}}
 ],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]};
}
async function openOffset(ru){
 await page.getByRole("menuitem",{name:ru?"Инструменты":"Tools",exact:true}).click();
 await page.getByRole("menuitem",{name:ru?"Сместить профиль":"Offset profile",exact:true}).click();
 const dialog=page.getByRole("dialog",{name:ru?"Сместить профиль":"Offset profile",exact:true});
 await expect(dialog.getByText(ru?"Исходный профиль: Profile · profile":"Source profile: Profile · profile",{exact:true})).toBeVisible();return dialog;
}
try{
 browser=await chromium.connectOverCDP(endpoint);
 page=browser.contexts().flatMap(c=>c.pages()).find(p=>/tauri\.localhost/.test(p.url()));
 if(!page)throw Error("Isolated Tauri page missing");
 page.setDefaultTimeout(30000);page.on("pageerror",error=>report.errors.push({message:error.message,stack:error.stack}));
 await expect(page.locator(".dashboard")).toBeVisible();
 for(const [locale,theme,mode,offset] of [["ru","dark","rectangle",2],["ru","light","holes",-2],["en","dark","circle",2],["en","light","rounded",2]]){
  const ru=locale==="ru",item={locale,theme,mode,offsetMm:offset};report.cases.push(item);
  await home();const prefs=await invoke("get_ui_preferences");
  await invoke("set_ui_preferences",{value:{...prefs,"forma.locale":locale,"forma.theme":theme}});
  await page.evaluate(({locale,theme})=>{localStorage.setItem("forma.locale",locale);localStorage.setItem("forma.theme",theme);},{locale,theme});
  await page.reload();await expect(page.locator("html")).toHaveAttribute("lang",locale);await expect(page.locator("html")).toHaveAttribute("data-theme",theme);
  const baseline=await seed("M4 offset "+locale+" "+theme,fixture(mode));
  item.before=await invoke("inspect_model",{projectId:baseline.id,revisionId:baseline.currentRevision});
  await openProject(baseline);await page.locator(".viewer-area canvas").hover();
  let dialog=await openOffset(ru);
  const label=ru?"Смещение профиля (мм)":"Profile offset (mm)",build=ru?"Построить":"Build";
  for(const value of ["0",""]){await dialog.getByLabel(label,{exact:true}).fill(value);await expect(dialog.getByRole("alert")).toBeVisible();await expect(dialog.getByRole("button",{name:build,exact:true})).toBeDisabled();}
  await dialog.getByLabel(label,{exact:true}).fill(String(mode==="holes"?4:mode==="circle"?-12:-11));
  await expect(dialog.getByText(ru?"Смещение разрушает контур или отверстие. Уменьшите величину смещения.":"The offset destroys a contour or hole. Reduce the offset magnitude.",{exact:true})).toBeVisible();
  await expect(dialog.getByRole("button",{name:build,exact:true})).toBeDisabled();
  expect((await projectById(baseline.id)).currentRevision).toBe(baseline.currentRevision);
  await dialog.getByRole("button",{name:ru?"Отмена":"Cancel",exact:true}).click();await expect(dialog).toBeHidden();
  expect((await projectById(baseline.id)).currentRevision).toBe(baseline.currentRevision);item.cancelDidNotCommit=true;
  dialog=await openOffset(ru);await dialog.getByLabel(label,{exact:true}).fill(String(offset));
  if(mode==="rounded"){
   await dialog.getByRole("combobox").nth(1).click();await page.locator('.select-menu[data-state="open"] .select-option[data-value="rounded"]').click();
  }
  await expect(dialog.getByText(ru?"Геометрия черновика корректна":"Draft geometry is valid",{exact:true})).toBeVisible();
  await expect(dialog.getByRole("button",{name:build,exact:true})).toBeEnabled();
  const canvas=dialog.locator(".model-preview-canvas canvas");await canvas.hover();await page.bringToFront();
  await expect.poll(async()=>foregroundPixels(await canvas.screenshot())).toBeGreaterThan(200);
  await page.screenshot({path:path.join(path.dirname(output),"offset-"+locale+"-"+theme+".png")});
  await dialog.getByRole("button",{name:build,exact:true}).click();await page.getByRole("button",{name:ru?"Разрешить один раз":"Allow once",exact:true}).click();
  await expect(page.locator(".viewer-area")).not.toHaveAttribute("data-displayed-revision",baseline.currentRevision);await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
  const committed=await projectById(baseline.id),doc=JSON.parse(committed.revisions.find(r=>r.id===committed.currentRevision).program),original=JSON.parse(baseline.revisions.find(r=>r.id===baseline.currentRevision).program);
  expect(committed.revisions.length).toBe(baseline.revisions.length+1);expect(doc.features.slice(0,original.features.length)).toEqual(original.features);expect(doc.features).toHaveLength(original.features.length+2);
  expect(committed.revisions.find(r=>r.id===baseline.currentRevision).program).toBe(baseline.revisions.find(r=>r.id===baseline.currentRevision).program);
  expect(doc.features.at(-2).operation).toEqual({type:"offsetProfile",sketchId:"profile",offset:literal(offset),join:mode==="rounded"?"rounded":"intersection"});
  expect(doc.features.at(-1).operation.distance).toEqual(original.features[1].operation.distance);
  item.revisionsAdded=1;item.totals=await invoke("inspect_model",{projectId:committed.id,revisionId:committed.currentRevision});
  const pi=Math.PI,expected=mode==="rectangle"?{volume:10560,area:3472,faces:6}:mode==="holes"?{volume:4360,area:2392,faces:10}:mode==="circle"?{volume:1440*pi,area:528*pi,faces:3}:{volume:10400+40*pi,area:3280+48*pi,faces:10};
  expect(item.totals.volumeMm3).toBeCloseTo(expected.volume,3);expect(item.totals.areaMm2).toBeCloseTo(expected.area,3);expect(item.totals.faceCount).toBe(expected.faces);
  item.history={};
  for(const [direction,index,volume] of [["undo",0,item.before.volumeMm3],["redo",1,expected.volume]]){
   const previous=(await projectById(baseline.id)).currentRevision;
   await page.locator(".model-history-controls button").nth(index).click();await page.getByRole("button",{name:ru?"Разрешить один раз":"Allow once",exact:true}).click();
   await expect(page.locator(".viewer-area")).not.toHaveAttribute("data-displayed-revision",previous);await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
   const moved=await projectById(baseline.id);expect(moved.revisions.some(r=>r.id===committed.currentRevision)).toBe(true);
   const properties=await invoke("inspect_model",{projectId:moved.id,revisionId:moved.currentRevision});expect(properties.volumeMm3).toBeCloseTo(volume,3);item.history[direction]={properties,offsetRevisionRetained:true};
  }
  await home();
 }
 if(report.errors.length)throw Error("Native WebView JavaScript errors");report.status="PASS";
}catch(error){report.status="FAIL";report.failure=error.stack??String(error);if(page)await page.screenshot({path:path.join(path.dirname(output),"offset-failure.png")}).catch(()=>{});process.exitCode=1;}
finally{fs.writeFileSync(output,JSON.stringify(report,null,2));console.log(JSON.stringify({status:report.status,cases:report.cases.length,failure:report.failure??null}));if(browser)await browser.close();}
