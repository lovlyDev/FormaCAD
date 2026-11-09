import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import { baseDocument, viewportTransformWorker } from "./viewportTransformFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");
const compound = {
  ...baseDocument,
  features: [...baseDocument.features, {
    id: "pattern", name: "Pattern", operation: {
      type: "patternLinear", bodyFeatureId: "pad", repeatHole: false,
      layout: { countU: 2, countV: 1, stepUMm: [50, 0, 0], stepVMm: [0, 0, 0] },
    },
  }],
  bodies: [{ ...baseDocument.bodies[0], sourceFeatureId: "pattern" }],
};

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`separate solids (${locale}, ${theme})`, async ({ page }) => {
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
      }, { base: { ...worker!.build(compound, id), revisionId: id }, theme });
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase", "ready");
      const partPoint = () => page.evaluate(() => (window as any).__m2.bodyScreen(2 / 9));
      await expect.poll(partPoint).not.toBeNull();
      const tools = page.getByRole("menuitem", { name: ru ? "Инструменты" : "Tools", exact: true });
      const separate = page.getByRole("menuitem", { name: ru ? "Разделить тела P" : "Separate solids P", exact: true });
      await tools.click();
      await expect(separate).toBeDisabled();
      await tools.click();
      await page.getByRole("button", { name: "Body", exact: true }).click();
      await expect.poll(() => page.evaluate(() => (window as any).__m2.selection())).toEqual(["body"]);
      await tools.click();
      await expect(separate).toBeEnabled();
      if (ru) {
        await tools.click();
        const point = await partPoint();
        await page.mouse.click(point.x, point.y);
        await expect.poll(() => page.evaluate(() => (window as any).__m2.selection())).toEqual(["body"]);
        await page.keyboard.press("p");
      } else {
        await separate.click();
      }
      await expect(page.getByRole("dialog")).toBeVisible();
      await page.getByRole("button", { name: ru ? "Разрешить один раз" : "Allow once", exact: true }).click();
      await expect.poll(() => page.evaluate(() => (window as any).__m2.project().currentRevision)).not.toBe(id);
      const commands = await page.evaluate(() => (window as any).__m2.calls
        .filter((call: any) => call.command === "apply_ir_commands").map((call: any) => call.args.commands));
      expect(commands).toMatchObject([[{ command: "separate_solids", bodyId: "body" }]]);
      expect(commands[0][0].newBodies).toHaveLength(1);
      const partId = commands[0][0].newBodies[0].id;
      await expect.poll(() => page.evaluate((partId) => {
        const { bounds } = (window as any).__m2;
        const original = bounds();
        const part = bounds(partId);
        return original && part && part[0] - original[0];
      }, partId)).toBeCloseTo(50, 3);
    });
  }
}
