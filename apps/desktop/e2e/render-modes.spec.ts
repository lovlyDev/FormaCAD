import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
import { selectValue } from "./customSelect";

function nestedCadFixture() {
  const worker = path.resolve("src-tauri/target/debug/forma-cad-worker.exe");
  if (process.platform !== "win32" || !fs.existsSync(worker)) return null;
  const root = path.resolve("../../.local"), stage = fs.mkdtempSync(path.join(root, "e2e-inner-geometry-"));
  const isolated = path.join(stage, "forma-cad-worker.exe"); fs.copyFileSync(worker, isolated);
  const mm = (value: number) => ({ kind: "literal", mm: value });
  const document = { schemaVersion: 2, revisionId: "appearance", parameters: [], features: [
    { id: "outer_profile", name: "Outer profile", operation: { type: "rectangle", width: mm(40), depth: mm(40) } },
    { id: "outer_pad", name: "Outer pad", operation: { type: "extrude", sketchId: "outer_profile", distance: mm(40) } },
    { id: "inner_profile", name: "Inner profile", operation: { type: "rectangle", width: mm(8), depth: mm(8) } },
    { id: "inner_pad", name: "Inner pad", operation: { type: "extrude", sketchId: "inner_profile", distance: mm(8) } },
    { id: "inner_offset", name: "Inner offset", operation: { type: "translate", bodyFeatureId: "inner_pad", offsetMm: [0, 0, 12] } },
  ], bodies: [{ id: "outer", name: "Outer", sourceFeatureId: "outer_pad" }, { id: "inner", name: "Inner", sourceFeatureId: "inner_offset" }] };
  execFileSync(isolated, [], { cwd: stage, input: JSON.stringify({ protocolVersion: 1, requestId: "nested", document }), windowsHide: true, timeout: 120000,
    env: { ...process.env, PATH: `${path.resolve("../../.local/occt-build/win64/vc14/bin")};${path.resolve("src-tauri/resources/occt")};${process.env.PATH ?? ""}` } });
  const result = JSON.parse(fs.readFileSync(path.join(stage, "result.json"), "utf8"));
  if (result.status !== "completed") throw new Error(JSON.stringify(result));
  const step = path.join(stage, "model.step"), hash = () => crypto.createHash("sha256").update(fs.readFileSync(step)).digest("hex");
  return { glb: `data:model/gltf-binary;base64,${fs.readFileSync(path.join(stage, "preview.glb")).toString("base64")}`, hash, originalHash: hash() };
}
const fixture = nestedCadFixture();
for (const locale of ["ru", "en"] as const) test(`X-Ray and Ghost ${locale}: real enclosed CAD geometry, export and cleanup`, async ({ page }, info) => {
  test.skip(!fixture, "Requires the current local debug CAD worker");
  const errors: string[] = []; page.on("pageerror", error => errors.push(error.message));
  await page.route("**/src/main.tsx", route => route.fulfill({ contentType: "application/javascript", body: "export {};" }));
  await page.addInitScript(locale => localStorage.setItem("forma.locale", locale), locale);
  await page.goto("/");
  await page.evaluate(async glb => { const { mountRenderModeHarness } = await import("/e2e/renderModeHarness.tsx"); await mountRenderModeHarness(glb); }, fixture!.glb);
  const mode = page.getByRole("combobox", { name: "Render mode" });
  await expect.poll(() => page.evaluate(() => window.__appearanceInspection?.firstOccluder)).toBe("outer");
  const camera = await page.evaluate(() => window.__appearanceInspection.camera);
  const baselineExport = await page.evaluate(() => window.__appearanceExport());
  for (const theme of ["dark", "light"]) {
    await page.evaluate(theme => window.__appearanceApplyTheme(theme), theme);
    await selectValue(page, mode, "xray");
    await expect(mode).toContainText(locale === "ru" ? "Рентген" : "X-Ray");
    await expect.poll(() => page.evaluate(() => window.__appearanceInspection.meshes.inner.opacity)).toBe(.12);
    const xray = await page.evaluate(() => window.__appearanceInspection.meshes);
    for (const body of [xray.inner, xray.outer]) {
      expect(body.depthWrite).toBe(false); expect(body.sourceOpacity).toBe(1);
      expect(body.outlines).toHaveLength(1); expect(body.outlines[0].depthTest).toBe(false);
    }
    await page.getByTestId("appearance-harness").screenshot({ path: info.outputPath(`xray-${locale}-${theme}.png`) });
    await selectValue(page, mode, "ghost");
    await expect(mode).toContainText(locale === "ru" ? "Приглушить окружение" : "Ghost");
    for (const selected of ["inner", "outer"]) {
      await page.getByTestId(`select-${selected}`).click();
      await expect.poll(() => page.evaluate(selected => window.__appearanceInspection.meshes[selected].opacity, selected)).toBe(1);
      const state = await page.evaluate(() => window.__appearanceInspection);
      expect(state.meshes[selected].color).toBe(theme === "light" ? "7c3aed" : "a78bfa");
      expect(state.meshes[selected === "inner" ? "outer" : "inner"].opacity).toBe(.09);
      expect(state.camera).toEqual(camera);
      await page.getByTestId("appearance-harness").screenshot({ path: info.outputPath(`ghost-${selected}-${locale}-${theme}.png`) });
    }
    // Export source provenance is independent of both theme and the active display state.
    expect(await page.evaluate(() => window.__appearanceExport())).toEqual(baselineExport);
    await page.getByTestId("toggle-section").click();
    await expect.poll(() => page.evaluate(() => window.__appearanceInspection.meshes.outer.clipping)).toBe(1);
    // A retained section can briefly own the source material during Model remount.
    await page.getByTestId("toggle-model").click();
    await expect.poll(() => page.evaluate(() => window.__appearanceInspection.meshes.outer.sourceRestored && window.__appearanceInspection.meshes.outer.clipping === 1)).toBe(true);
    await page.getByTestId("toggle-model").click();
    await expect.poll(() => page.evaluate(() => !window.__appearanceInspection.meshes.outer.sourceRestored && window.__appearanceInspection.meshes.outer.clipping === 1)).toBe(true);
    for (const selected of ["inner", "outer"]) {
      await page.getByTestId(`select-${selected}`).click();
      await expect.poll(() => page.evaluate(() => Object.values(window.__appearanceInspection.meshes).every((body: any) => body.clipping === 1))).toBe(true);
    }
    await page.evaluate(theme => window.__appearanceApplyTheme(theme === "dark" ? "light" : "dark"), theme);
    await expect.poll(() => page.evaluate(() => Object.values(window.__appearanceInspection.meshes).every((body: any) => body.clipping === 1))).toBe(true);
    await page.evaluate(theme => window.__appearanceApplyTheme(theme), theme);
    await selectValue(page, mode, "xray");
    await expect.poll(() => page.evaluate(() => window.__appearanceInspection.meshes.inner.clipping)).toBe(1);
    await page.getByTestId("toggle-section").click();
    await expect.poll(() => page.evaluate(() => Object.values(window.__appearanceInspection.meshes).every((body: any) => body.clipping === 0))).toBe(true);
    await page.getByTestId("toggle-model").click();
    await expect.poll(() => page.evaluate(() => Object.values(window.__appearanceInspection.meshes).every((body: any) => body.sourceRestored && body.outlines.length === 0))).toBe(true);
    await page.getByTestId("toggle-model").click();
    await expect.poll(() => page.evaluate(() => window.__appearanceInspection.meshes.inner.outlines.length)).toBe(1);
  }
  expect(fixture!.hash()).toBe(fixture!.originalHash);
  expect(errors).toEqual([]);
});
