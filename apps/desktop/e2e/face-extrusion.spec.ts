import { expect, test } from "@playwright/test";
import { baseDocument, viewportTransformWorker } from "./viewportTransformFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");
const baseline = "8d4e2f9c-0f6b-4a8e-9a43-3c1f5b7f0a11";

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`face extrusion by keyboard and arrow (${locale}, ${theme})`, async ({ page }) => {
      const ru = locale === "ru";
      await page.exposeFunction("__m2Build", (document: unknown, id: string) => worker!.build(document, id));
      await page.exposeFunction("__m3Measure", (id: string, body: string, query: unknown) =>
        worker!.measure(id, body, query));
      await page.route(/\/src\/main\.tsx(?:\?.*)?$/, (route) => route.fulfill({
        contentType: "application/javascript", body: "export {};",
      }));
      await page.addInitScript((value) => localStorage.setItem("forma.locale", value), locale);
      await page.goto("/");
      await page.evaluate(async ({ base, theme }) => {
        const { mountViewportTransformHarness } = await import("/e2e/viewportTransformHarness.tsx");
        await mountViewportTransformHarness(base);
        (window as any).__m2.theme(theme);
      }, { base: { ...worker!.build(baseDocument, baseline), revisionId: baseline }, theme });
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase", "ready");
      await expect.poll(() => page.evaluate(() => (window as any).__m2.topFaceScreen())).not.toBeNull();
      const viewport = page.locator('[data-editor="viewport3d"]');
      await viewport.focus();
      await page.keyboard.press("Digit2");
      const point = await page.evaluate(() => (window as any).__m2.topFaceScreen());
      await page.mouse.click(point.x, point.y);
      await expect.poll(() => page.evaluate(() => (window as any).__m2.face()?.topologyRef?.role))
        .toBe("box-face:zmax");

      const dialog = page.getByRole("dialog", { name: ru ? "Вытянуть грань" : "Extrude face" });
      const distance = dialog.getByLabel(ru ? "Расстояние вытягивания (мм)" : "Extrusion distance (mm)");
      const build = dialog.getByRole("button", { name: ru ? "Построить" : "Build", exact: true });
      await page.keyboard.press("KeyE");
      await expect(dialog).toBeVisible();
      await distance.fill("0");
      await expect(build).toBeDisabled();
      await page.keyboard.press("Escape");
      await expect(dialog).toHaveCount(0);

      const arrow = page.getByRole("button", { name: ru ? "Потяните, чтобы вытянуть грань" : "Drag to extrude face" });
      await expect(arrow).toBeVisible();
      const box = (await arrow.boundingBox())!;
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2 - 35, { steps: 4 });
      await page.mouse.up();
      await expect(dialog).toBeVisible();
      await expect(distance).not.toHaveValue("");
      await distance.fill(ru ? "-2,5" : "2.5");
      await build.click();
      await page.getByRole("button", { name: ru ? "Разрешить один раз" : "Allow once", exact: true }).click();
      await expect.poll(() => page.evaluate(() => (window as any).__m2.project().currentRevision))
        .not.toBe(baseline);
      await expect.poll(async () => (await page.evaluate(() => (window as any).__m2.bounds()))?.[4])
        .toBeCloseTo(ru ? 7.5 : 12.5, 3);
      const commands = await page.evaluate(() => (window as any).__m2.calls
        .filter((call: any) => call.command === "apply_ir_commands").map((call: any) => call.args.commands));
      expect(commands).toMatchObject([[
        { command: "add_feature", feature: { operation: { type: "extrudeFace", distance: { mm: ru ? -2.5 : 2.5 } } } },
        { command: "set_body_source", bodyId: "body" },
      ]]);
    });
  }
}
