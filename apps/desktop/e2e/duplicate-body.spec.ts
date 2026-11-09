import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import { baseDocument, viewportTransformWorker } from "./viewportTransformFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`duplicate body through the Command API (${locale}, ${theme})`, async ({ page }) => {
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
      await page.keyboard.press("Shift+d");
      const label = ru ? "Дублировать тело" : "Duplicate body";
      const dialog = page.getByRole("dialog", { name: label });
      const submit = dialog.getByRole("button", { name: label, exact: true });
      const name = dialog.getByLabel(ru ? "Имя копии" : "Copy name");
      const offset = dialog.getByLabel(ru ? "Смещение копии X (мм)" : "Copy offset X (mm)");
      await name.fill("");
      await expect(submit).toBeDisabled();
      await name.fill("Copy fixture");
      await offset.fill(ru ? "50,5" : "50.5");
      await submit.click();
      await page.getByRole("button", { name: ru ? "Разрешить один раз" : "Allow once", exact: true }).click();
      await expect.poll(() => page.evaluate(() => (window as any).__m2.project().currentRevision)).not.toBe(id);
      const commands = await page.evaluate(() => (window as any).__m2.calls
        .filter((call: any) => call.command === "apply_ir_commands").map((call: any) => call.args.commands));
      expect(commands).toMatchObject([[
        { command: "duplicate_body", bodyId: "body", name: "Copy fixture", offsetMm: [50.5, 0, 0] },
      ]]);
      const copyId = commands[0][0].newBodyId;
      await expect.poll(() => page.evaluate((copyId) => {
        const { bounds } = (window as any).__m2;
        const original = bounds();
        const copy = bounds(copyId);
        return original && copy && copy[0] - original[0];
      }, copyId)).toBeCloseTo(50.5, 3);
    });
  }
}
