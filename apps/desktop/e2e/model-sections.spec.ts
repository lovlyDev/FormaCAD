import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
import { selectValue } from "./customSelect";

function nativeSectionFixture() {
  const worker = path.resolve("src-tauri/target/debug/forma-cad-worker.exe");
  if (process.platform !== "win32" || !fs.existsSync(worker)) return null;
  const root = path.resolve("../../.local"), stage = fs.mkdtempSync(path.join(root, "e2e-native-sections-"));
  const isolated = path.join(stage, "forma-cad-worker.exe"); fs.copyFileSync(worker, isolated);
  const document = JSON.parse(fs.readFileSync(path.resolve("../../docs/fixtures/parameterized-disc.cad.json"), "utf8"));
  const env = { ...process.env, PATH: `${path.resolve("../../.local/occt-build/win64/vc14/bin")};${path.resolve("src-tauri/resources/occt")};${process.env.PATH ?? ""}` };
  const model = path.join(stage, "model"); fs.mkdirSync(model);
  execFileSync(isolated, [], { cwd: model, input: JSON.stringify({ protocolVersion: 1, requestId: "section-fixture", document }), env, windowsHide: true, timeout: 120000 });
  const metrics = JSON.parse(fs.readFileSync(path.join(model, "result.json"), "utf8"));
  if (metrics.status !== "completed") throw new Error(JSON.stringify(metrics));
  const step = fs.readFileSync(path.join(model, "model.step")), sourceSha256 = crypto.createHash("sha256").update(step).digest("hex");
  let requests = 0;
  return { document, sourceSha256, sourceSize: step.length, metrics, glb: `data:model/gltf-binary;base64,${fs.readFileSync(path.join(model, "preview.glb")).toString("base64")}`,
    section(plane: { originMm: number[]; normal: number[]; deflectionMm: number }) {
      const cwd = path.join(stage, `section-${requests++}`); fs.mkdirSync(path.join(cwd, "inputs"), { recursive: true });
      fs.writeFileSync(path.join(cwd, "inputs", `step-${sourceSha256}.step`), step);
      execFileSync(isolated, [], { cwd, input: JSON.stringify({ protocolVersion: 1, requestId: `section-${requests}`, operation: "section_step", sourceSha256, sectionPlane: plane }), env, windowsHide: true, timeout: 120000 });
      const result = JSON.parse(fs.readFileSync(path.join(cwd, "result.json"), "utf8"));
      if (result.status !== "completed") throw new Error(JSON.stringify(result));
      const bytes = fs.readFileSync(path.join(cwd, "section.json"));
      expect(crypto.createHash("sha256").update(bytes).digest("hex")).toBe(result.sectionSha256);
      const report = JSON.parse(bytes.toString("utf8"));
      expect(report.sourceSha256).toBe(sourceSha256);
      return report;
    } };
}
const fixture = nativeSectionFixture();

for (const locale of ["ru", "en"] as const) test(`native sections ${locale}: exact curves, axes, clipping restoration, repeated motion and themes`, async ({ page }, info) => {
  test.skip(!fixture, "Requires the current local debug CAD worker with section support");
  const labels = locale === "ru" ? { title: "Сечение модели", clip: "Отсекать модель плоскостью сечения", offset: "Смещение плоскости, мм", axis: "Плоскость сечения" }
    : { title: "Section view", clip: "Clip model with section plane", offset: "Plane offset in mm", axis: "Section plane" };
  const failures: string[] = []; page.on("pageerror", error => failures.push(error.message));
  await page.route("**/src/main.tsx", route => route.fulfill({ contentType: "application/javascript", body: "export {};" }));
  let calls = 0;
  await page.route("**/__exact_section", route => {
    const args = route.request().postDataJSON();
    expect(args.expectedRevision).toBe(fixture!.document.revisionId);
    expect(args.projectId).toMatch(/^[\w-]+$/);
    calls++;
    const { originMm, normal, deflectionMm } = args;
    return route.fulfill({ contentType: "application/json", body: JSON.stringify(fixture!.section({ originMm, normal, deflectionMm })) });
  });
  await page.addInitScript(locale => {
    localStorage.setItem("forma.locale", locale);
    window.isTauri = true;
    window.__TAURI_INTERNALS__ = { metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } }, invoke: async (command: string, args: unknown) => command === "section_model"
      ? (await fetch("/__exact_section", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(args) })).json() : null };
  }, locale);
  await page.goto("/");
  await page.evaluate(async data => { const { mountSectionHarness } = await import("/e2e/sectionHarness.tsx"); await mountSectionHarness(data); }, { document: fixture!.document, glb: fixture!.glb, sourceSha256: fixture!.sourceSha256, sourceSize: fixture!.sourceSize });
  await expect.poll(() => page.evaluate(() => window.__sectionState?.available)).toBe(true);
  await expect.poll(() => page.evaluate(() => window.__sectionInspection?.lowerHits ?? 0)).toBeGreaterThan(0);
  const lower = await page.evaluate(() => window.__sectionInspection.lowerHits);
  const worldBounds = await page.evaluate(() => window.__sectionInspection.worldBounds);
  expect(worldBounds[0]).toBeCloseTo(fixture!.metrics.boundsMm[0], 3);
  expect(worldBounds[1]).toBeCloseTo(fixture!.metrics.boundsMm[2], 3);
  expect(worldBounds[2]).toBeCloseTo(fixture!.metrics.boundsMm[1], 3);
  await page.getByTestId("open-section").click();
  const panel = page.locator(".section-controls");
  await expect(panel).toBeVisible();
  // The GLB is float32; the initial midpoint must not expose representation noise.
  await expect(panel.getByRole("spinbutton", { name: labels.offset })).toHaveValue("4");
  await expect.poll(() => panel.getByRole("combobox",{name:labels.axis,exact:true}).evaluate(element => element.getBoundingClientRect().height)).toBe(32);
  expect(await panel.getByRole("spinbutton", { name: labels.offset }).evaluate(element => element.getBoundingClientRect().height)).toBe(32);
  expect(await panel.evaluate(element => getComputedStyle(element).animationDuration)).toBe("0.2s");
  const checkbox = panel.getByRole("checkbox"); await expect(checkbox).toBeEnabled(); await checkbox.check();
  await expect.poll(() => page.evaluate(() => !!window.__sectionState?.report)).toBe(true);
  await expect.poll(() => page.evaluate(() => window.__sectionInspection.lowerHits)).toBe(0);
  await expect.poll(() => page.evaluate(() => window.__sectionInspection.upperHits)).toBeGreaterThan(0);
  const clipped = await page.evaluate(() => window.__sectionInspection);
  expect(clipped.clippingEnabled).toBe(true); expect(clipped.allRaycastsRestored).toBe(false);
  expect(clipped.clipPlaneCount).toBe(clipped.meshCount);
  await expect.poll(() => page.evaluate(() => window.__sectionInspection.positions.length)).toBeGreaterThan(5);
  const mapped = await page.evaluate(() => ({ report: window.__sectionState.report, points: window.__sectionInspection.positions }));
  const curve = mapped.report.geometry.curves[0]; expect(curve.pointsMm.length).toBeGreaterThan(1);
  const expected = [curve.pointsMm[0][0], curve.pointsMm[0][2], -curve.pointsMm[0][1], curve.pointsMm[1][0], curve.pointsMm[1][2], -curve.pointsMm[1][1]];
  expected.forEach((value, i) => expect(mapped.points[i]).toBeCloseTo(value, 4));
  const camera = clipped.camera;
  for (const theme of ["dark", "light"]) {
    await page.evaluate(theme => window.__sectionApplyTheme(theme), theme);
    await expect.poll(() => page.evaluate(() => window.__sectionInspection.color)).toBe(theme === "dark" ? "c4b5fd" : "6d28d9");
    expect(await page.evaluate(() => window.__sectionInspection.camera)).toEqual(camera);
    await page.getByTestId("section-harness").screenshot({ path: info.outputPath(`sections-${locale}-${theme}.png`) });
  }
  // Exercise the production axis selector, not just the pure coordinate helper.
  for (const [axis, cadNormal, worldNormal] of [
    ["xz", [0, 1, 0], [0, 0, -1]],
    ["yz", [1, 0, 0], [1, 0, 0]],
    ["xy", [0, 0, 1], [0, 1, 0]],
  ] as const) {
    await selectValue(page, panel.getByRole("combobox",{name:labels.axis,exact:true}), axis);
    await expect.poll(() => page.evaluate(() => window.__sectionState.report?.geometry.plane.normal)).toEqual(cadNormal);
    const normal = await page.evaluate(() => window.__sectionInspection.clippingNormal);
    worldNormal.forEach((value, i) => expect(normal[i]).toBeCloseTo(value, 6));
    await expect.poll(() => page.evaluate(() => window.__sectionInspection.positions.length)).toBeGreaterThan(5);
    const { report, points } = await page.evaluate(() => ({ report: window.__sectionState.report, points: window.__sectionInspection.positions }));
    const start = report.geometry.curves[0].pointsMm[0];
    [start[0], start[2], -start[1]].forEach((value, i) => expect(points[i]).toBeCloseTo(value, 4));
    expect(await page.evaluate(() => window.__sectionInspection.camera)).toEqual(camera);
  }
  for (let i = 0; i < 2; i++) {
    await panel.locator("summary").click(); await expect(panel).toHaveAttribute("data-open", "false");
    await expect(panel).toHaveCount(0);
    await page.getByTestId("open-section").click(); await expect(panel).toBeVisible();
    expect(await panel.evaluate(element => getComputedStyle(element).animationDuration)).toBe("0.2s");
    expect(await panel.locator("summary svg").last().evaluate(element => getComputedStyle(element).transitionDuration)).toBe("0.26s");
    expect(await panel.locator(".editor-disclosure-body").getAttribute("data-animation-count")).not.toBeNull();
  }
  await checkbox.uncheck();
  await expect.poll(() => page.evaluate(() => window.__sectionInspection.clippingEnabled)).toBe(false);
  await expect.poll(() => page.evaluate(() => window.__sectionInspection.allRaycastsRestored && window.__sectionInspection.allMaterialsRestored)).toBe(true);
  expect(await page.evaluate(() => window.__sectionInspection.clipPlaneCount)).toBe(0);
  expect(await page.evaluate(() => window.__sectionInspection.lowerHits)).toBe(lower);
  expect(await page.evaluate(() => window.__sectionInspection.positions.length)).toBe(0);
  expect(calls).toBe(4);
  expect(await panel.locator("summary").innerText()).toContain(labels.title);
  await expect(checkbox).toHaveAccessibleName(labels.clip);
  expect(await page.evaluate(async () => { const { useWorkspace } = await import("/src/stores/workspace.ts"); return JSON.stringify(useWorkspace.getState().project) === window.__sectionBaseline; })).toBe(true);
  expect(failures).toEqual([]);
});
