import { expect, test } from "@playwright/test";
import { nativePreviewFixture } from "./nativePreviewFixture";
const nativeFixture = nativePreviewFixture();
const fixture = nativeFixture?.document, glb = nativeFixture?.glb ?? "";
for (const locale of ["ru","en"] as const) test(`editor and solid+edges selections in ${locale}`, async ({ page }, info) => {
 test.skip(!nativeFixture,"Requires the locally built Windows native CAD worker");
 await page.addInitScript(({locale,glb,fixture}) => {
  localStorage.setItem("forma.locale",locale);
  const now=new Date().toISOString();
  localStorage.setItem("forma.projects.v1",JSON.stringify([{schemaVersion:1,id:"b3a2c1d0-0000-4000-8000-000000000001",name:"Disc",units:"mm",agent:"codex",pinned:false,createdAt:now,updatedAt:now,currentRevision:"r1",messages:[],exports:[],files:[{name:"preview.glb",kind:"model",size:1,data:glb}],revisions:[{id:"r1",parent:null,createdAt:now,prompt:"Disc",source:"disc.step",preview:"preview.glb",program:JSON.stringify(fixture),parameters:{kind:"blank",width:60,depth:60,height:60,thickness:5,holeDiameter:8,holes:1}}]}]));
 },{locale,glb,fixture});
 await page.goto("/#/project/b3a2c1d0-0000-4000-8000-000000000001");
 await expect(page.locator("html")).toHaveAttribute("lang",locale);
 const tr=await page.evaluate(async()=>{const {t}=await import("/src/i18n/index.ts");return Object.fromEntries(["Select CAD face","Select CAD edge","Edit model parameters","Named CAD parameters","Закрыть"].map(k=>[k,t(k)]));});
 const canvas=page.locator("canvas"); await expect(page.getByRole("button",{name:tr["Select CAD face"],exact:true})).toBeEnabled();
 await page.waitForTimeout(900); const box=(await canvas.boundingBox())!;
 await page.getByRole("button",{name:tr["Select CAD face"],exact:true}).click();
 await page.mouse.click(box.x+box.width*.5,box.y+box.height*.54);
 await expect.poll(()=>page.evaluate(async()=>{const {useWorkspace}=await import("/src/stores/workspace.ts");return useWorkspace.getState().selectedFace?.faceOrdinal??null;})).not.toBeNull();
 await canvas.screenshot({path:info.outputPath(`face-${locale}.png`)});
 // A point far from any edge must not choose a nearby edge merely because it is nearer the camera.
 await page.getByRole("button",{name:tr["Select CAD edge"],exact:true}).click();
 await page.mouse.click(box.x+box.width*.5,box.y+box.height*.54);
 expect(await page.evaluate(async()=>{const {useWorkspace}=await import("/src/stores/workspace.ts");return useWorkspace.getState().selectedEdge;})).toBeNull();
 const edgePoint=await page.evaluate(async glb=>{
  const THREE=await import("/node_modules/.vite/deps/three.js");const {loadModel}=await import("/src/lib/files.ts"); const {disposeModel}=await import("/src/lib/model.ts");
  const root=await loadModel({name:"disc.glb",kind:"model",size:1,data:glb});const canvas=document.querySelector("canvas")!;const box=canvas.getBoundingClientRect();const snapshot=JSON.parse(canvas.dataset.camera!);
  const camera=new THREE.PerspectiveCamera(38,box.width/box.height,.1,10000);camera.position.fromArray(snapshot.position);camera.quaternion.fromArray(snapshot.quaternion);camera.updateMatrixWorld();
  const {cadEdges}=await import("/src/features/viewer/edgeSelection.ts");const {pickCadEdge}=await import("/src/features/viewer/pickCadEdge.ts");
  let result=null;
  root.traverse(node=>{if(result||!(node instanceof THREE.Mesh))return;cadEdges(node)?.forEach((edge,index)=>{
   if(result||edge.points.length<2)return;const a=new THREE.Vector3(...edge.points[0]),b=new THREE.Vector3(...edge.points[1]);a.applyMatrix4(node.matrixWorld);b.applyMatrix4(node.matrixWorld);const top=new THREE.Box3().setFromObject(root).max.y;
   if(Math.abs(a.y-b.y)>1e-4||Math.abs(a.y-top)>1e-4)return;const point=a.lerp(b,.5).project(camera);
   const x=(point.x+1)*box.width/2,y=(1-point.y)*box.height/2;const hit=pickCadEdge(root,camera,{x,y},box.width,box.height);
   if(hit?.edgeOrdinal===index+1)result={x:box.x+x,y:box.y+y,ordinal:index+1};
  });});disposeModel(root);return result;
 },glb);expect(edgePoint).not.toBeNull();await page.mouse.click(edgePoint!.x,edgePoint!.y);
 await expect.poll(()=>page.evaluate(async()=>{const {useWorkspace}=await import("/src/stores/workspace.ts");return useWorkspace.getState().selectedEdge?.edgeOrdinal;})).toBe(edgePoint!.ordinal);
 await canvas.screenshot({path:info.outputPath(`edge-${locale}.png`)});
 const renderMenu=page.locator(".render-select [role=combobox]");const triggerBox=(await renderMenu.boundingBox())!;await renderMenu.click();const menu=page.getByRole("listbox");await expect(menu).toBeVisible();expect(Math.abs((await menu.boundingBox())!.x-triggerBox.x)).toBeLessThan(2);expect(Math.abs((await menu.boundingBox())!.x-(await page.locator(".render-select").boundingBox())!.x)).toBeLessThan(2);await page.keyboard.press("Escape");
 await page.getByRole("button",{name:tr["Edit model parameters"],exact:true}).click();
 await expect(page.locator(".edge-fillet-editor")).toBeVisible();await expect(page.locator(".edge-fillet-controls button")).toBeDisabled();
 for (const theme of ["dark","light"] as const) {
  await page.evaluate(async theme=>{const {applyTheme}=await import("/src/lib/theme.ts");applyTheme(theme);},theme);
  await page.locator(".model-editor-modal").screenshot({path:info.outputPath(`editor-${locale}-${theme}.png`)});
 }
 await page.setViewportSize({width:700,height:850}); await page.locator(".model-editor-modal").screenshot({path:info.outputPath(`editor-${locale}-narrow.png`)});
 await page.getByRole("button",{name:tr["Закрыть"],exact:true}).click();
 const roundtrip=await page.evaluate(async glb=>{
  const {loadModel,exportMesh}=await import("/src/lib/files.ts");const {disposeModel}=await import("/src/lib/model.ts");
  const original=await loadModel({name:"disc.glb",kind:"model",size:1,data:glb});
  const blob=await exportMesh(original,"glb");const url=URL.createObjectURL(blob);
  const imported=await loadModel({name:"roundtrip.glb",kind:"model",size:blob.size,data:url});URL.revokeObjectURL(url);
  const {hasCadTopology}=await import("/src/lib/cadTopologyTransport.ts");const valid=hasCadTopology(imported)&&imported.userData.formaNativeCadPreview;
  disposeModel(original);disposeModel(imported);return valid;
 },glb); expect(roundtrip).toBe(true);
});
