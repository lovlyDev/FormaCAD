import { selectValue } from "./customSelect";
import { expect,test } from "@playwright/test";
for(const locale of ["ru","en"] as const){
 test(`profiles, point constraints and named dimensions in ${locale} and both themes`,async({page},testInfo)=>{
  await page.addInitScript(locale=>localStorage.setItem("forma.locale",locale),locale);await page.goto("/");
  const id=await page.evaluate(async()=>{
   const {createStarterSketch}=await import("/src/lib/sketchDocument.ts");const id=crypto.randomUUID(),rid=crypto.randomUUID(),now=new Date().toISOString();
   localStorage.setItem("forma.projects.v1",JSON.stringify([{schemaVersion:1,id,name:"Profile tools",units:"mm",agent:"codex",pinned:false,createdAt:now,updatedAt:now,currentRevision:rid,messages:[],files:[],exports:[],revisions:[{id:rid,parent:null,createdAt:now,prompt:"Sketch",program:createStarterSketch(),parameters:{kind:"box",width:40,depth:20,height:10,thickness:2,holeDiameter:0,holes:0}}]}]));return id;
  });await page.goto(`/#/project/${id}`);await page.reload();
  const t=await page.evaluate(async()=>{const {t}=await import("/src/i18n/index.ts");return Object.fromEntries(["Edit model parameters","Edit 2D sketch","CAD source","Add rectangle contour","Named CAD parameters","New parameter name","New parameter value in mm","Add parameter","length","Length in mm","Dimension source {{value0}}","Parameter {{value0}} in mm","Remove parameter {{value0}}","Numeric dimension","First coincident point","Second coincident point","Add coincidence","Fix point {{value0}}","Remove contour {{value0}}","Split edge {{value0}}","Remove point {{value0}}"].map(key=>[key,t(key,{value0:"TOKEN"})]));});
  const label=(key:string,id="")=>t[key].replace("TOKEN",id);
  await page.getByRole("button",{name:label("Edit model parameters"),exact:true}).click();await page.getByText(label("Edit 2D sketch"),{exact:true}).click();
  const source=async()=>JSON.parse(await page.getByLabel(label("CAD source"),{exact:true}).inputValue());const original=await source();
  await page.getByRole("button",{name:label("Add rectangle contour"),exact:true}).click();expect((await source()).features[0].operation.points).toHaveLength(8);
  await page.getByText(label("Named CAD parameters"),{exact:true}).click();await page.getByLabel(label("New parameter name"),{exact:true}).fill("Pocket width");await page.getByLabel(label("New parameter value in mm"),{exact:true}).fill("8");await page.getByRole("button",{name:label("Add parameter"),exact:true}).click();
  const parameter=(await source()).parameters.at(-1);
  await page.locator(".sketch-editor-lines > div").filter({hasText:"line_ab"}).getByRole("button",{name:label("length"),exact:true}).click();
  const constraint=(await source()).features[0].operation.constraints.at(-1);await selectValue(page,page.getByLabel(label("Dimension source {{value0}}",constraint.id),{exact:true}),parameter.id);
  await expect(page.getByRole("button",{name:label("Remove parameter {{value0}}",parameter.id),exact:true})).toBeDisabled();
  await page.getByLabel(label("Parameter {{value0}} in mm",parameter.id),{exact:true}).fill("12");expect((await source()).parameters.at(-1).valueMm).toBe(12);
  await selectValue(page,page.getByLabel(label("Dimension source {{value0}}",constraint.id),{exact:true}),"");expect((await source()).features[0].operation.constraints.at(-1).distance.mm).toBe(12);
  await page.getByRole("button",{name:label("Fix point {{value0}}","point_a"),exact:true}).click();
  await selectValue(page,page.getByLabel(label("First coincident point"),{exact:true}),"point_a");await selectValue(page,page.getByLabel(label("Second coincident point"),{exact:true}),"point_c");await page.getByRole("button",{name:label("Add coincidence"),exact:true}).click();
  const draft=await source();expect(draft.features[0].operation.constraints.at(-1).kind).toBe("coincident");
  await expect.poll(async()=>{const before=await page.locator("canvas").getAttribute("data-camera");await page.waitForTimeout(500);return before===await page.locator("canvas").getAttribute("data-camera");}).toBe(true);
  const camera=await page.locator("canvas").getAttribute("data-camera");
  for(const theme of ["dark","light"] as const){await page.evaluate(async theme=>{const {applyTheme}=await import("/src/lib/theme.ts");applyTheme(theme);},theme);await expect(page.locator("html")).toHaveAttribute("data-theme",theme);expect(await source()).toEqual(draft);expect(await page.locator("canvas").getAttribute("data-camera")).toEqual(camera);await page.locator(".sketch-editor-profile-tools").screenshot({path:testInfo.outputPath(`profile-${locale}-${theme}.png`)});await page.locator(".cad-parameter-editor").screenshot({path:testInfo.outputPath(`parameters-${locale}-${theme}.png`)});}
  const loopId=draft.features[0].operation.lines.find((line:{id:string})=>line.id.startsWith("profile_line_")).id;
  await page.getByRole("button",{name:label("Remove contour {{value0}}",loopId),exact:true}).click();expect((await source()).features[0].operation.points).toHaveLength(4);
  await page.getByRole("button",{name:label("Split edge {{value0}}","line_ab"),exact:true}).click();expect((await source()).features[0].operation.points).toHaveLength(5);
  const added=(await source()).features[0].operation.points.find((p:{id:string})=>!original.features[0].operation.points.some((old:{id:string})=>old.id===p.id));
  await page.getByRole("button",{name:label("Remove point {{value0}}",added.id),exact:true}).click();expect((await source()).features[0].operation.points).toHaveLength(4);
  await page.getByRole("button",{name:label("Remove parameter {{value0}}",parameter.id),exact:true}).click();expect((await source()).parameters.some((p:{id:string})=>p.id===parameter.id)).toBe(false);
 });
}
