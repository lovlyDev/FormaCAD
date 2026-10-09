import { expect,test } from "@playwright/test";
import { nativePreviewFixture } from "./nativePreviewFixture";
const fixture=nativePreviewFixture();
for(const locale of ["ru","en"] as const)test(`GLB import creates a populated project and protects the model in ${locale}`,async({page})=>{
 test.skip(!fixture,"Requires the locally built Windows native CAD worker");
 await page.addInitScript(locale=>localStorage.setItem("forma.locale",locale),locale);await page.goto("/");await expect(page.locator("html")).toHaveAttribute("lang",locale);
 await page.locator("input[type=file]").first().setInputFiles({name:"disc.glb",mimeType:"model/gltf-binary",buffer:Buffer.from(fixture!.glb.split(",")[1],"base64")});
 await expect(page.getByText(locale==="ru"?"Создать проект из выбранных файлов":"Create a project from selected files",{exact:true})).toBeVisible();await expect(page.getByText("disc.glb",{exact:true})).toBeVisible();
 await page.getByRole("button",{name:locale==="ru"?"Создать проект":"Create project",exact:true}).click();
 await expect(page.getByRole("button",{name:locale==="ru"?"Выбрать CAD-грань":"Select CAD face",exact:true})).toBeEnabled();
 expect(await page.evaluate(async()=>{const {useWorkspace}=await import("/src/stores/workspace.ts");const p=useWorkspace.getState().project!;return {source:p.revisions[0].source,count:p.revisions.length};})).toEqual({source:"disc.glb",count:1});
 await page.getByRole("button",{name:locale==="ru"?"Изменить параметры модели":"Edit model parameters",exact:true}).click();
 await expect(page.getByRole("button",{name:locale==="ru"?"Построить":"Build",exact:true})).toBeDisabled();await expect(page.getByRole("button",{name:locale==="ru"?"Создать 2D-эскиз":"Create 2D sketch",exact:true})).toHaveCount(0);
});
