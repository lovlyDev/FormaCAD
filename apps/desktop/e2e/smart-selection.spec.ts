import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import { baseDocument, viewportTransformWorker } from "./viewportTransformFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`linked, similar and tangent selection (${locale}, ${theme})`, async ({ page }) => {
      const id = randomUUID();
      const ru = locale === "ru";
      await page.exposeFunction("__m3Measure", (revision: string, body: string, query: unknown) =>
        worker!.measure(revision, body, query));
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
      const viewport = page.locator('[data-editor="viewport3d"]');
      await viewport.focus();
      await page.keyboard.press("Digit2");
      const face = await facePoint();
      await page.mouse.click(face.x, face.y);
      await expect.poll(() => page.evaluate(() => (window as any).__m2.face()?.topologyRef?.role))
        .toBe("box-face:zmax");
      await page.keyboard.press("Shift+G");
      await expect(page.getByText(ru ? "Выбрано элементов: 2" : "Selected elements: 2")).toBeVisible();
      await page.keyboard.press("KeyL");
      await expect(page.getByRole("button", { name: ru ? "Выбрать связанное тело" : "Select linked body" })).toHaveCount(0);
      expect(await page.evaluate(() => (window as any).__m2.selection())).toEqual(["body"]);
      await page.keyboard.press("Digit3");
      const role = "box-edge:x:ymin:zmax";
      const edgePoint = () => page.evaluate((role) => (window as any).__m2.edgeScreen(role), role);
      await expect.poll(edgePoint).not.toBeNull();
      const edge = await edgePoint();
      await page.keyboard.down("Alt");
      await page.mouse.click(edge.x, edge.y);
      await page.keyboard.up("Alt");
      await expect.poll(() => page.evaluate(() => (window as any).__m2.calls
        .filter((call: any) => call.command === "measure_model_reference")
        .map((call: any) => call.args.query.mode).filter(Boolean))).toEqual(["similarFaces", "tangentChain"]);
      await expect(page.getByText(ru ? "Выбрано элементов: 1" : "Selected elements: 1")).toBeVisible();
      expect(await page.evaluate(() => (window as any).__m2.project().currentRevision)).toBe(id);
    });
  }
}
