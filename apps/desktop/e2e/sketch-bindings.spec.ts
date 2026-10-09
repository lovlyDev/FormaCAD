import { expect,test } from "@playwright/test";
for(const locale of ["ru","en"] as const){
 test(`linked base and triangle keep effective geometry, source and themes in ${locale}`,async({page},info)=>{
  await page.addInitScript(locale=>localStorage.setItem("forma.locale",locale),locale);await page.goto("/");
  await page.evaluate(async()=>{
   const React=await import("/node_modules/.vite/deps/react.js"),DOM=await import("/node_modules/.vite/deps/react-dom_client.js"),{ProgramDialog}=await import("/src/app/dialogs/ProgramDialog.tsx"),{createLinkedProfiles}=await import("/src/lib/templates/linkedProfiles.ts"),{useWorkspace,newProject}=await import("/src/stores/workspace.ts");
   useWorkspace.getState().setProject(newProject("Linked test","blank","mm","codex"));
   DOM.default.createRoot(document.body.appendChild(document.createElement("div"))).render(React.default.createElement(ProgramDialog,{open:true,close:()=>{},source:createLinkedProfiles(),disabled:false,nativeCadAvailable:true,nativeCadChecked:true,onApply:()=>{throw new Error("No commit");}}));
  });
  const ru=locale==="ru",sketch=page.locator(".sketch-editor").first();await sketch.locator("summary").click();
  const width=page.getByLabel(ru?"Параметр width в мм":"Parameter width in mm",{exact:true});await width.fill("80");
  await expect(sketch.getByLabel(ru?"X точки a":"Point a X",{exact:true})).toHaveValue("-40");await expect(sketch.getByLabel(ru?"X точки a":"Point a X",{exact:true})).toBeDisabled();
  const triangle=page.locator(".sketch-editor").nth(1);await triangle.locator("summary").click();await expect(triangle.getByLabel(ru?"Начало плоскости X":"Plane origin X",{exact:true})).toHaveValue("-40");
  const source=page.getByLabel(ru?"Исходник CAD-модели":"CAD source",{exact:true});let doc=JSON.parse(await source.inputValue());expect(doc.features[0].operation.points[0].xMm).toBe(-20);expect(doc.features[0].operation.bindings).toHaveLength(8);
  await sketch.getByRole("button",{name:ru?"Удалить связь координаты a_x":"Unlink coordinate a_x",exact:true}).click();await expect(sketch.getByLabel(ru?"X точки a":"Point a X",{exact:true})).toBeEnabled();doc=JSON.parse(await source.inputValue());expect(doc.features[0].operation.points[0].xMm).toBe(-40);expect(doc.features[0].operation.bindings).toHaveLength(7);
  for(const theme of ["dark","light"]){await page.evaluate(async theme=>{const {applyTheme}=await import("/src/lib/theme.ts");applyTheme(theme);},theme);await sketch.locator(".sketch-links").screenshot({path:info.outputPath(`links-${locale}-${theme}.png`)});expect(await sketch.locator(".sketch-links").evaluate(el=>getComputedStyle(el).borderTopStyle)).toBe("solid");}
  expect(await page.evaluate(async()=>{const {useWorkspace}=await import("/src/stores/workspace.ts");return useWorkspace.getState().project!.revisions.length;})).toBe(0);
 });
}
