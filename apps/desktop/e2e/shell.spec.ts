import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import { baseDocument, viewportTransformWorker } from "./viewportTransformFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`shell with two openings (${locale}, ${theme})`, async ({ page }) => {
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
      const facePoint = () => page.evaluate(() => (window as any).__m2.topFaceScreen());
      await expect.poll(facePoint).not.toBeNull();
      await page.locator('[data-editor="viewport3d"]').focus();
      await page.keyboard.press("Digit2");
      const point = await facePoint();
      await page.mouse.click(point.x, point.y);
      await expect.poll(() => page.evaluate(() => (window as any).__m2.face()?.topologyRef?.role))
        .toBe("box-face:zmax");
      await page.getByRole("menuitem", { name: ru ? "Инструменты" : "Tools", exact: true }).click();
      await page.getByRole("menuitem", { name: ru ? "Оболочка" : "Shell", exact: true }).click();
      const dialog = page.getByRole("dialog", { name: ru ? "Оболочка" : "Shell" });
      await expect(dialog.getByRole("checkbox", { name: "box-face:zmax" })).toBeChecked();
      await dialog.getByRole("checkbox", { name: "box-face:zmin" }).check();
      if (!ru) await dialog.getByRole("checkbox", { name: "Build walls outward" }).check();
      const thickness = dialog.getByRole("textbox");
      const build = dialog.getByRole("button", { name: ru ? "Построить" : "Build", exact: true });
      await thickness.fill("0");
      await expect(build).toBeDisabled();
      await thickness.fill(ru ? "1,0" : "1.0");
      await build.click();
      await page.getByRole("button", { name: ru ? "Разрешить один раз" : "Allow once", exact: true }).click();
      await expect.poll(() => page.evaluate(() => (window as any).__m2.project().currentRevision)).not.toBe(id);
      await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase", "ready");
      await expect.poll(async () => {
        const bounds = await page.evaluate(() => (window as any).__m2.bounds());
        return bounds && bounds[3] - bounds[0];
      }).toBeCloseTo(ru ? 40 : 42, 3);
      const batches = await page.evaluate(() => (window as any).__m2.calls
        .filter((call: any) => call.command === "apply_ir_commands").map((call: any) => call.args.commands));
      expect(batches).toMatchObject([[
        { command: "add_feature", feature: { operation: {
          type: "shell", offset: { mm: ru ? -1 : 1 },
          openings: expect.arrayContaining([
            expect.objectContaining({ role: "box-face:zmax" }),
            expect.objectContaining({ role: "box-face:zmin" }),
          ]),
        } } },
        { command: "set_body_source", bodyId: "body" },
      ]]);
    });
  }
}
