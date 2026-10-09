import { expect, test, type Page } from "@playwright/test";
import { modelApplyFixture } from "./modelApplyFixture";

const fixture = modelApplyFixture();
async function mount(page: Page, locale: "ru" | "en") {
  page.on("pageerror", error => console.log("APP ERROR", error.stack));
  await page.route(/\/src\/main\.tsx(?:\?.*)?$/, route => route.fulfill({ contentType: "application/javascript", body: "export {};" }));
  await page.addInitScript(locale => localStorage.setItem("forma.locale", locale), locale);
  await page.goto("/");
  await page.evaluate(async fixture => {
    const { mountModelApplyHarness } = await import("/e2e/modelApplyHarness.tsx");
    await mountModelApplyHarness(fixture);
  }, fixture!);
  await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase", "ready");
  await expect(page.locator(".viewer-area canvas")).toBeVisible();
  // A mesh can enter the scene before the first CameraControl frame. Wait for
  // the initial preset's centered target, not an arbitrary timeout.
  await expect.poll(()=>page.evaluate(()=>{
    const saved=(window as any).__applyHarness.camera();if(!saved)return false;
    const camera=JSON.parse(saved);return camera.target[1]>0;
  })).toBe(true);
}
async function editAndReview(page: Page, locale: "ru" | "en") {
  await page.getByRole("button", { name: locale === "ru" ? "Изменить параметры модели" : "Edit model parameters", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await dialog.locator("summary").filter({ hasText: locale === "ru" ? "Исходник CAD-модели" : "CAD source" }).click();
  await dialog.locator("textarea.program-editor").fill(fixture!.candidate);
  await dialog.getByRole("button", { name: locale === "ru" ? "Построить" : "Build", exact: true }).click();
  await expect(page.getByRole("button", { name: locale === "ru" ? "Разрешить один раз" : "Allow once" })).toBeVisible();
}
async function allow(page: Page, locale: "ru" | "en") {
  await page.getByRole("button", { name: locale === "ru" ? "Разрешить один раз" : "Allow once" }).click();
}
for (const locale of ["ru", "en"] as const) {
  test(`actual App ${locale}: committed GLB swap is atomic and failure retains the real previous geometry`, async ({ page }, info) => {
    test.skip(!fixture, "Requires the local native worker");
    const errors: string[] = []; page.on("pageerror", error => errors.push(error.message));
    await mount(page, locale);
    await expect.poll(() => page.evaluate(() => (window as any).__applyHarness.inspect()?.vertices)).toBeGreaterThan(0);
    const baseline=await page.evaluate(()=>(window as any).__applyHarness.inspect());
    for (const theme of ["dark", "light"]) {
      await page.evaluate(theme => (window as any).__applyHarness.theme(theme), theme);
      await page.screenshot({path:info.outputPath(`baseline-${locale}-${theme}.png`)});
      expect((await page.evaluate(()=>(window as any).__applyHarness.inspect())).uuid).toBe(baseline.uuid);
    }
    await page.evaluate(()=>(window as any).__applyHarness.section());
    await expect.poll(()=>page.evaluate(()=>(window as any).__applyHarness.inspect()?.clipping)).toBe(1);
    await page.evaluate(() => (window as any).__applyHarness.setDelayed(true));
    await editAndReview(page, locale); await allow(page, locale);
    await expect.poll(() => page.evaluate(() => (window as any).__applyHarness.filePending())).toBe(true);
    const viewer = page.locator(".viewer-area");
    await expect(viewer).toHaveAttribute("data-target-revision", "accepted");
    await expect(viewer).toHaveAttribute("data-displayed-revision", "base");
    await expect(viewer).toHaveAttribute("data-scene-phase", "loading");
    await expect(viewer).toContainText(locale==="ru"?"Показаны модель и свойства ревизии 1.":"Displayed model and properties: revision 1.");
    await expect(page.locator(".inspector")).toContainText(locale==="ru"?"8,00":"8.00");
    const retained=await page.evaluate(()=>(window as any).__applyHarness.inspect());
    expect(retained.uuid).toBe(baseline.uuid);expect(retained.clipping).toBe(1);
    expect(retained.camera).toEqual(baseline.camera);
    expect(await page.evaluate(()=>(window as any).__applyHarness.calls.filter((call:any)=>call.command==="section_model"&&call.args.expectedRevision==="accepted"))).toHaveLength(0);
    await expect(page.getByRole("button",{name:locale==="ru"?"Выбрать CAD-ребро":"Select CAD edge"})).toBeDisabled();
    await page.screenshot({ path: info.outputPath(`retained-${locale}.png`) });
    await page.evaluate(() => (window as any).__applyHarness.fail());
    await expect(viewer).toHaveAttribute("data-scene-phase", "failed");
    await expect(viewer).toHaveAttribute("data-displayed-revision", "base");
    await expect(viewer.getByRole("alert")).toContainText(locale === "ru" ? "предыдущая корректная модель" : "previous valid model");
    await page.evaluate(() => (window as any).__applyHarness.setDelayed(false));
    await viewer.getByRole("button", { name: locale === "ru" ? "Повторить загрузку модели" : "Retry preview loading" }).click();
    await expect(viewer).toHaveAttribute("data-displayed-revision", "accepted");
    await expect(viewer).toHaveAttribute("data-scene-phase", "ready");
    const accepted=await page.evaluate(()=>(window as any).__applyHarness.inspect());
    expect(accepted.uuid).not.toBe(baseline.uuid);expect(accepted.bounds[0]).toBeGreaterThan(baseline.bounds[0]);
    await page.evaluate(()=>(window as any).__applyHarness.emitRevision());
    await expect.poll(()=>page.evaluate(()=>(window as any).__applyHarness.inspect()?.uuid)).toBe(accepted.uuid);
    const calls = await page.evaluate(() => (window as any).__applyHarness.calls.filter((call: any) => call.command === "apply_program"));
    expect(calls).toHaveLength(1); expect(calls[0].args.projectId).toBe("A");
    expect(calls[0].args.expectedRevision).toBe("base");
    await page.screenshot({ path: info.outputPath(`accepted-${locale}.png`) });
    expect(errors).toEqual([]);
  });
  test(`actual App ${locale}: no-op and rejected kernel output preserve scene and saved draft`,async({page})=>{
    test.skip(!fixture,"Requires the local native worker"); await mount(page,locale);
    await expect.poll(()=>page.evaluate(()=>(window as any).__applyHarness.inspect()?.vertices)).toBeGreaterThan(0);
    const baseline=await page.evaluate(()=>(window as any).__applyHarness.inspect());
    await page.evaluate(()=>(window as any).__applyHarness.setOutcome("noop"));
    await editAndReview(page,locale);await allow(page,locale);
    await expect.poll(()=>page.evaluate(()=>(window as any).__applyHarness.calls.filter((call:any)=>call.command==="apply_program").length)).toBe(1);
    expect(await page.evaluate(()=>(window as any).__applyHarness.inspect())).toEqual(baseline);
    expect(await page.evaluate(()=>(window as any).__applyHarness.project().currentRevision)).toBe("base");
    await page.evaluate(()=>(window as any).__applyHarness.setOutcome("error"));
    // Reopening uses the persisted candidate and disclosure state.
    await page.getByRole("button",{name:locale==="ru"?"Изменить параметры модели":"Edit model parameters",exact:true}).click();
    await expect(page.locator("textarea.program-editor")).toHaveValue(fixture!.candidate);
    await page.getByRole("dialog").getByRole("button",{name:locale==="ru"?"Построить":"Build",exact:true}).click();
    await allow(page,locale);await expect(page.locator(".error-toast")).toBeVisible();
    expect(await page.evaluate(()=>(window as any).__applyHarness.project().currentRevision)).toBe("base");
    expect(await page.evaluate(()=>(window as any).__applyHarness.inspect())).toEqual(baseline);
    expect(await page.evaluate(()=>(window as any).__applyHarness.project().revisions.length)).toBe(1);
    await page.evaluate(()=>(window as any).__applyHarness.emitRevision());
    expect(await page.evaluate(()=>(window as any).__applyHarness.listenerCount())).toBe(1);
    await page.evaluate(()=>(window as any).__applyHarness.unmount());
    await expect.poll(()=>page.evaluate(()=>(window as any).__applyHarness.listenerCount())).toBe(0);
  });
  test(`actual App ${locale}: stale approval never retargets another project`, async ({ page }) => {
    test.skip(!fixture, "Requires the local native worker");
    await mount(page, locale); await editAndReview(page, locale);
    await page.evaluate(() => (window as any).__applyHarness.switchProject());
    await allow(page, locale);
    await expect.poll(() => page.evaluate(() => (window as any).__applyHarness.project().id)).toBe("B");
    expect(await page.evaluate(() => (window as any).__applyHarness.calls.filter((call: any) => call.command === "apply_program"))).toHaveLength(0);
    await expect(page.locator(".error-toast")).not.toBeVisible();
  });
  test(`actual App ${locale}: delayed agent failure in A never writes an error into B`,async({page})=>{
    test.skip(!fixture,"Requires the local native worker");await mount(page,locale);
    await page.locator(".composer textarea").fill("Resize the part");
    await page.locator(".send-button").click();await allow(page,locale);
    await expect.poll(()=>page.evaluate(()=>(window as any).__applyHarness.planPending())).toBe(true);
    await page.evaluate(()=>{(window as any).__applyHarness.switchProject();(window as any).__applyHarness.rejectPlan();});
    await expect.poll(()=>page.evaluate(()=>(window as any).__applyHarness.project().id)).toBe("B");
    await expect(page.locator(".error-toast")).not.toBeVisible();
    expect(await page.evaluate(()=>(window as any).__applyHarness.project().messages)).toEqual([]);
    expect(await page.evaluate(()=>(window as any).__applyHarness.calls.filter((call:any)=>call.command==="save_project"&&call.args.project.id==="B"))).toHaveLength(0);
  });
}
