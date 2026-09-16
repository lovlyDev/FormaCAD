import { expect, test } from "@playwright/test";
test("workspace survives reload with camera, draft, hidden body and display mode",async({page})=>{
  await page.addInitScript(()=>localStorage.setItem("forma.locale","en"));
  await page.goto("/");
  const id=await page.evaluate(()=>{
    const id=crypto.randomUUID(),rid=crypto.randomUUID(),now=new Date().toISOString();
    localStorage.setItem("forma.projects.v1",JSON.stringify([{id,name:"Persistent model",schemaVersion:1,units:"mm",agent:"codex",pinned:false,createdAt:now,updatedAt:now,currentRevision:rid,messages:[],files:[],exports:[],revisions:[{id:rid,parent:null,createdAt:now,prompt:"Created",parameters:{kind:"box",width:60,depth:40,height:10,thickness:2,holeDiameter:0,holes:0}}]}]));return id;
  });
  await page.goto(`/#/project/${id}`);await page.reload();
  const canvas=page.locator("canvas");await expect(canvas).toBeVisible();
  await page.getByRole("textbox",{name:"Message your CAD assistant"}).fill("Keep this draft after updating");
  await page.getByRole("combobox",{name:"Rendering mode"}).click();
  await page.getByRole("option",{name:"Wireframe",exact:true}).click();
  await page.getByRole("button",{name:"Top",exact:true}).click();
  await canvas.hover();await page.mouse.wheel(0,-160);
  let prior="", stable=0;
  await expect.poll(async()=>{
    const value=await page.evaluate(key=>localStorage.getItem(key),`forma.ui.project.${id}.view.main.camera`);
    stable=value===prior ? stable+1 : 0;prior=value??"";return stable;
  },{intervals:[500],timeout:20000}).toBeGreaterThan(2);
  const camera=JSON.parse(prior);
  await page.getByRole("button",{name:"Hide Block",exact:true}).click();
  await page.reload();
  await expect(page.getByRole("textbox",{name:"Message your CAD assistant"})).toHaveValue("Keep this draft after updating");
  await expect(page.getByRole("button",{name:"Show Block",exact:true})).toBeVisible();
  await expect(page.getByRole("combobox",{name:"Rendering mode"})).toContainText("Wireframe");
  await expect.poll(async()=>{
    const state=JSON.parse(await canvas.getAttribute("data-camera")??"{}");
    return state.position?Math.hypot(...state.position.map((v:number,i:number)=>v-camera.position[i])):Infinity;
  }).toBeLessThan(0.01);
  await expect(page.locator(".avatar")).toHaveCount(0);
});
