import { expect,test } from "@playwright/test";
for(const locale of ["ru","en"] as const)test(`sketch drag commits once and editor preferences survive reopening in ${locale}`,async({page},info)=>{
 await page.addInitScript(locale=>localStorage.setItem("forma.locale",locale),locale);await page.goto("/");await expect(page.locator("html")).toHaveAttribute("lang",locale);
 await page.evaluate(async()=>{
  const React=await import("/node_modules/.vite/deps/react.js"),ReactDOM=await import("/node_modules/.vite/deps/react-dom_client.js");
  const {Sketch2dEditor}=await import("/src/components/Sketch2dEditor.tsx"),{createStarterSketch}=await import("/src/lib/sketchDocument.ts"),{newProject,useWorkspace}=await import("/src/stores/workspace.ts");
  useWorkspace.getState().setProject(newProject("Drag test","blank","mm","codex"));
  window.isTauri=true;window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:"main"},currentWebview:{label:"main"}},invoke:async(command:string,args:any)=>{if(command==="set_ui_preferences")(window as any).savedPreferences=args.value;return null;}};
  let operation=JSON.parse(createStarterSketch()).features[0].operation;const host=document.body.appendChild(document.createElement("div"));host.style.cssText="position:fixed;inset:20px;background:var(--bg);overflow:auto;z-index:1000";
  const root=ReactDOM.default.createRoot(host);(window as any).commits=0;
  const render=()=>root.render(React.default.createElement(Sketch2dEditor,{operation,disabled:false,featureId:"drag",onChange:(next:any)=>{(window as any).commits++;operation=next;render();}}));render();
  (window as any).reopen=()=>{root.render(null);setTimeout(render,30);};
 });
 const details=page.locator(".sketch-editor");await details.locator("summary").click();const svg=details.locator("svg.sketch-editor-canvas");await expect(svg).toBeVisible();await page.waitForTimeout(250);
 for(let pass=0;pass<3;pass++){const body=details.locator(":scope > .editor-disclosure-body"),before=Number(await body.getAttribute("data-animation-count")??0);await details.locator("summary").click();await expect(details).not.toHaveAttribute("open","");await details.locator("summary").click();await expect.poll(async()=>Number(await body.getAttribute("data-animation-count")??0)).toBe(before+2);await page.waitForTimeout(250);}
 const view=await svg.getAttribute("viewBox"),circle=svg.locator("circle").first();await circle.scrollIntoViewIfNeeded();const box=(await circle.boundingBox())!,x=box.x+box.width/2,y=box.y+box.height/2;
 await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x+65,y+25,{steps:20});await expect(svg).toHaveAttribute("data-dragging","true");expect(await page.evaluate(()=>(window as any).commits)).toBe(0);expect(await svg.getAttribute("viewBox")).toBe(view);const position=await circle.getAttribute("cx");await page.mouse.up();expect(await page.evaluate(()=>(window as any).commits)).toBe(1);expect(await circle.getAttribute("cx")).toBe(position);expect(await svg.getAttribute("viewBox")).toBe(view);
 const next=(await circle.boundingBox())!;await page.mouse.move(next.x+next.width/2,next.y+next.height/2);await page.mouse.down();await page.mouse.move(next.x+40,next.y+40,{steps:8});await page.keyboard.press("Escape");await page.mouse.up();expect(await circle.getAttribute("cx")).toBe(position);expect(await page.evaluate(()=>(window as any).commits)).toBe(1);
 const solved=details.locator(".sketch-solved-preview-toggle input[type=checkbox]");if(await solved.count()){await solved.check();}
 await page.evaluate(async()=>{const {flushPreferences}=await import("/src/lib/persistence.ts");await flushPreferences();});
 const stored=await page.evaluate(()=>(window as any).savedPreferences);expect(Object.keys(stored).some(key=>key.endsWith("sections.sketch.drag")&&stored[key]==="true")).toBe(true);
 await page.evaluate(()=>(window as any).reopen());await expect(details).toHaveAttribute("open","");expect(await svg.getAttribute("viewBox")).toBe(view);
 for(const theme of ["dark","light"]){await page.evaluate(async theme=>{const {applyTheme}=await import("/src/lib/theme.ts");applyTheme(theme);},theme);await svg.screenshot({path:info.outputPath(`drag-${locale}-${theme}.png`)});}
});
