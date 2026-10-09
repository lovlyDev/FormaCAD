import { expect,test } from "@playwright/test";
test("constrained geometry cannot build a surprising shape without an explicit choice",async({page})=>{
 await page.goto("/");await page.evaluate(async()=>{
 const React=await import("/node_modules/.vite/deps/react.js"),DOM=await import("/node_modules/.vite/deps/react-dom_client.js"),{ProgramDialog}=await import("/src/app/dialogs/ProgramDialog.tsx"),{createStarterSketch}=await import("/src/lib/sketchDocument.ts");
 const doc=JSON.parse(createStarterSketch());doc.features[0].operation.points[1].yMm=-17;doc.features[0].operation.constraints=[{id:"horizontal",kind:"horizontal",lineId:"line_ab"}];
 window.isTauri=true;window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:"main"},currentWebview:{label:"main"}},invoke:async(command:string,args:any)=>command==="analyze_sketch"?{status:"solved",errorCode:null,degreesOfFreedom:7,redundantEquations:0,maxResidualMm:0,constraints:[],solvedPoints:args.operation.points.map((p:any)=>({...p,yMm:p.id==="point_b"?-10:p.yMm})),loopCount:1,holeCount:0,profileAreaMm2:800}:null};
 DOM.default.createRoot(document.body.appendChild(document.createElement("div"))).render(React.default.createElement(ProgramDialog,{open:true,close:()=>{},source:JSON.stringify(doc),disabled:false,nativeCadAvailable:true,nativeCadChecked:true,onApply:()=>{throw new Error("No commit");}}));
 });
 const build=page.getByRole("button",{name:"Построить",exact:true});await expect(build).toBeDisabled();await expect(page.locator(".sketch-review-actions")).toBeVisible();
 await page.locator(".sketch-editor summary").click();await expect(page.locator(".sketch-solved-outline")).toHaveCount(4);
 await page.locator(".sketch-review-actions button").first().click();await expect(page.locator(".sketch-build-review")).toHaveCount(0);const doc=JSON.parse(await page.getByLabel("Исходник CAD-модели",{exact:true}).inputValue());expect(doc.features[0].operation.points[1].yMm).toBe(-17);expect(doc.features[0].operation.constraints).toHaveLength(0);expect(doc.features[1].operation.sketchId).toBe("sketch_1");
});
