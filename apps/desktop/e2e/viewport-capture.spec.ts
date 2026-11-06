import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import { baseDocument, viewportTransformWorker } from "./viewportTransformFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`scaled transparent viewport snapshot (${locale}, ${theme})`, async ({ page }) => {
      const ru = locale === "ru";
      const id = randomUUID();
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
      const canvas = page.locator(".viewer-area canvas");
      await expect(canvas).toHaveAttribute("data-background", /^#/);
      const dimensions = () => canvas.evaluate((element) => {
        const canvas = element as HTMLCanvasElement;
        return [canvas.width, canvas.height, canvas.dataset.background];
      });
      const before = await dimensions();
      const title = ru ? "Снимок вида" : "Viewport snapshot";
      await page.getByRole("button", { name: title, exact: true }).click();
      const dialog = page.getByRole("dialog", { name: title });
      await dialog.getByRole("combobox", { name: ru ? "Масштаб PNG" : "PNG scale" }).selectOption("2");
      await dialog.getByRole("checkbox", { name: ru ? "Прозрачный фон" : "Transparent background" }).check();
      await dialog.getByRole("button", { name: ru ? "Прикрепить вид модели к чату" : "Attach viewport to chat" }).click();
      await page.getByRole("button", { name: ru ? "Разрешить один раз" : "Allow once", exact: true }).click();
      await expect.poll(() => page.evaluate(() => (window as any).__m2.project().files
        .filter((file: any) => file.name.startsWith("viewport-")).length)).toBe(1);
      const png = await page.evaluate(async () => {
        const project = (window as any).__m2.project();
        const data = project.files.find((file: any) => file.name.startsWith("viewport-")).data;
        const image = await createImageBitmap(await (await fetch(data)).blob());
        const output = new OffscreenCanvas(image.width, image.height);
        const ctx = output.getContext("2d")!;
        ctx.drawImage(image, 0, 0);
        const alpha = ctx.getImageData(0, 0, 1, 1).data[3];
        image.close();
        return { width: output.width, height: output.height, alpha, revisionId: project.currentRevision };
      });
      expect(png).toEqual({ width: Number(before[0]) * 2, height: Number(before[1]) * 2, alpha: 0, revisionId: id });
      expect(await dimensions()).toEqual(before);
    });
  }
}
