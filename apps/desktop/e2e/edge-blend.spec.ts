import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import { baseDocument, viewportTransformWorker } from "./viewportTransformFixture";
import { buildPreviewPacket } from "./viewportPreviewFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");
const role = "cylinder-edge:top";
const secondRole = "cylinder-edge:bottom";
const cylinder = structuredClone(baseDocument);
Object.assign(cylinder.features[0], { operation: { type: "circle", radius: { kind: "literal", mm: 10 } } });
Object.assign(cylinder.features[1].operation, { distance: { kind: "literal", mm: 20 } });

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`edge fillet and chamfer (${locale}, ${theme})`, async ({ page }) => {
      const ru = locale === "ru";
      const id = randomUUID();
      await page.exposeFunction("__m2Build", (document: unknown, id: string) => worker!.build(document, id));
      await page.exposeFunction("__m3Preview", (source: string) => buildPreviewPacket(worker!, source));
      await page.route(/\/src\/main\.tsx(?:\?.*)?$/, (route) => route.fulfill({
        contentType: "application/javascript", body: "export {};",
      }));
      await page.addInitScript((value) => localStorage.setItem("forma.locale", value), locale);
      await page.goto("/");
      await page.evaluate(async ({ base, theme }) => {
        const { mountViewportTransformHarness } = await import("/e2e/viewportTransformHarness.tsx");
        await mountViewportTransformHarness(base);
        (window as any).__m2.theme(theme);
      }, { base: { ...worker!.build(cylinder, id), revisionId: id }, theme });
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase", "ready");
      const edgePoint = () => page.evaluate((role) => (window as any).__m2.edgeScreen(role), role);
      await expect.poll(edgePoint).not.toBeNull();
      await page.locator('[data-editor="viewport3d"]').focus();
      await page.keyboard.press("Digit3");
      const point = await edgePoint();
      await page.mouse.click(point.x, point.y);
      await expect.poll(() => page.evaluate(() => (window as any).__m2.edge()?.topologyRef?.role))
        .toBe(role);
      await page.keyboard.press("Control+Numpad7");
      await expect.poll(() => page.evaluate(() => (window as any).__m2.viewDirection()?.[1]))
        .toBe(1);
      const secondPoint = () => page.evaluate((role) => (window as any).__m2.edgeScreen(role), secondRole);
      await expect.poll(secondPoint).not.toBeNull();
      const second = await secondPoint();
      await page.keyboard.down("Shift");
      await page.mouse.click(second.x, second.y);
      await page.keyboard.up("Shift");
      await expect.poll(() => page.evaluate(() => (window as any).__m2.edge()?.topologyRef?.role))
        .toBe(secondRole);
      await page.keyboard.press("Control+b");
      const dialog = page.getByRole("dialog", { name: ru ? "Обработать выбранные рёбра" : "Blend selected edges" });
      await expect(dialog).toBeVisible();
      const mode = ru ? "fillet" : "chamfer";
      await dialog.getByLabel(ru ? "Обработка ребра" : "Edge treatment", { exact: true }).selectOption(mode);
      await dialog.getByRole("slider").press("ArrowRight");
      const distance = dialog.getByRole("textbox");
      await expect(distance).toHaveValue(ru ? "1,05" : "1.05");
      await distance.fill(ru ? "2,0" : "2.0");
      await expect(dialog.getByText(ru ? "Геометрия черновика корректна" : "Draft geometry is valid")).toBeVisible();
      await dialog.getByRole("button", { name: ru ? "Построить" : "Build", exact: true }).click();
      await page.getByRole("button", { name: ru ? "Разрешить один раз" : "Allow once", exact: true }).click();
      await expect.poll(() => page.evaluate(() => (window as any).__m2.project().currentRevision)).not.toBe(id);
      const batches = await page.evaluate(() => (window as any).__m2.calls
        .filter((call: any) => call.command === "apply_ir_commands").map((call: any) => call.args.commands));
      expect(batches).toMatchObject([[
        { command: "add_feature", feature: { operation: {
          type: ru ? "filletReferencedEdge" : "chamferReferencedEdge", reference: { role },
          additionalReferences: [{ role: secondRole }],
          [ru ? "radius" : "distance"]: { mm: 2 },
        } } },
        { command: "set_body_source", bodyId: "body" },
      ]]);
    });
  }
}
