import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import { baseDocument, viewportTransformWorker } from "./viewportTransformFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`pattern through the Command API (${locale}, ${theme})`, async ({ page }) => {
      const ru = locale === "ru";
      const id = randomUUID();
      await page.exposeFunction("__m2Build", (document: unknown, id: string) => worker!.build(document, id));
      await page.route(/\/src\/main\.tsx(?:\?.*)?$/, (route) => route.fulfill({
        contentType: "application/javascript", body: "export {};",
      }));
      await page.addInitScript((value) => localStorage.setItem("forma.locale", value), locale);
      await page.goto("/");
      await page.evaluate(async ({ base, theme }) => {
        const { mountViewportTransformHarness } = await import("/e2e/viewportTransformHarness.tsx");
        await mountViewportTransformHarness(base);
        (window as any).__m2.theme(theme);
      }, { base: { ...worker!.build(baseDocument, id), revisionId: id }, theme });
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase", "ready");
      await expect.poll(() => page.evaluate(() => (window as any).__m2.bodyScreen())).not.toBeNull();
      const point = await page.evaluate(() => (window as any).__m2.bodyScreen());
      await page.mouse.click(point.x, point.y);
      await expect.poll(() => page.evaluate(() => (window as any).__m2.selection())).toEqual(["body"]);
      await page.getByRole("menuitem", { name: ru ? "Инструменты" : "Tools", exact: true }).click();
      await page.getByRole("menuitem", { name: ru ? "Массив" : "Pattern", exact: true }).click();
      const dialog = page.getByRole("dialog", { name: ru ? "Массив" : "Pattern" });
      const build = dialog.getByRole("button", { name: ru ? "Построить" : "Build", exact: true });
      const count = dialog.getByLabel(ru ? "Количество экземпляров" : "Instance count");
      await count.fill("1");
      await expect(build).toBeDisabled();
      if (ru) {
        await count.fill("3");
        await dialog.getByLabel("Количество рядов").fill("2");
        await dialog.getByLabel("Второй шаг (мм)").fill("45,0");
      } else {
        await dialog.getByRole("combobox", { name: "Pattern layout" }).selectOption("circular");
        await count.fill("4");
        await dialog.getByLabel("Axis origin X (mm)").fill("50");
      }
      await build.click();
      await page.getByRole("button", { name: ru ? "Разрешить один раз" : "Allow once", exact: true }).click();
      await expect.poll(() => page.evaluate(() => (window as any).__m2.project().currentRevision)).not.toBe(id);
      await expect.poll(async () => {
        const bounds = await page.evaluate(() => (window as any).__m2.bounds());
        return bounds && bounds[3] - bounds[0];
      }).toBeCloseTo(140, 3);
      const commands = await page.evaluate(() => (window as any).__m2.calls
        .filter((call: any) => call.command === "apply_ir_commands").map((call: any) => call.args.commands));
      expect(commands).toMatchObject([[
        { command: "add_feature", feature: { operation: {
          type: ru ? "patternLinear" : "patternCircular", repeatHole: false,
          layout: ru ? { countU: 3, countV: 2, stepVMm: [0, 45, 0] } :
            { count: 4, axisOriginMm: [50, 0, 0], stepDeg: 90 },
        } } },
        { command: "set_body_source", bodyId: "body" },
      ]]);
    });
  }
}
