import { expect,test } from "@playwright/test";
for(const locale of ["ru","en"] as const){test(`RMB keeps the ordinary cursor and preserves camera orientation in ${locale}`,async({page})=>{
 await page.addInitScript(locale=>localStorage.setItem("forma.locale",locale),locale);await page.goto("/");
 await page.getByRole("button",{name:locale==="ru"?"Новый проект":"New project",exact:true}).click();await page.getByRole("button",{name:locale==="ru"?"Создать проект":"Create project",exact:true}).click();
 const canvas=page.locator("canvas");await expect.poll(async()=>JSON.parse(await canvas.getAttribute("data-camera")??"{}").position?.length??0).toBe(3);
 for(const projection of ["perspective","orthographic"]){
  if(projection==="orthographic")await page.getByRole("button",{name:locale==="ru"?"Переключить ортографическую проекцию":"Toggle orthographic projection"}).click();
  await page.waitForTimeout(400);const before=JSON.parse(await canvas.getAttribute("data-camera")??"{}");const box=(await canvas.boundingBox())!;const x=box.x+box.width*.5,y=box.y+box.height*.5;
  await page.mouse.move(x,y);await page.mouse.down({button:"right"});expect(await page.evaluate(()=>document.pointerLockElement)).toBeNull();
  const marker=page.locator(".locked-pan-anchor");await expect(marker).toHaveCount(0);await page.mouse.move(x+100,y+40,{steps:10});await page.waitForTimeout(100);await expect(marker).toHaveCount(0);
  const after=JSON.parse(await canvas.getAttribute("data-camera")??"{}");expect(Math.hypot(...after.target.map((v:number,i:number)=>v-before.target[i]))).toBeGreaterThan(.1);expect(after.zoom).toBe(before.zoom);after.quaternion.forEach((value:number,i:number)=>expect(value).toBeCloseTo(before.quaternion[i],12));
  await page.mouse.up({button:"right"});await expect.poll(()=>page.evaluate(()=>document.pointerLockElement===null)).toBe(true);await expect(marker).toHaveCount(0);
  const released=JSON.parse(await canvas.getAttribute("data-camera")??"{}");await page.mouse.move(x-50,y-20);await page.waitForTimeout(100);const idle=JSON.parse(await canvas.getAttribute("data-camera")??"{}");for(const key of ["position","target","quaternion"])idle[key].forEach((value:number,i:number)=>expect(value).toBeCloseTo(released[key][i],9));
  await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x+70,y+20,{steps:8});await page.mouse.up();await page.waitForTimeout(100);const rotated=JSON.parse(await canvas.getAttribute("data-camera")??"{}");expect(Math.hypot(...rotated.quaternion.map((v:number,i:number)=>v-idle.quaternion[i]))).toBeGreaterThan(.001);expect(await page.evaluate(()=>document.pointerLockElement)).toBeNull();
 }
});}
