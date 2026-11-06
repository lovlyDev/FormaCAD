import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import { baseDocument, viewportTransformWorker } from "./viewportTransformFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`copy a model operation as JSON (${locale}, ${theme})`, async ({ page, context }) => {
      const id = randomUUID();
      await context.grantPermissions(["clipboard-read", "clipboard-write"]);
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
      const row = page.getByRole("treeitem", { name: /Pad/ });
      if (theme === "dark") await row.click({ button: "right" });
      else await row.press("Shift+F10");
      const copy = page.getByRole("button", {
        name: locale === "ru" ? "Копировать как команду" : "Copy as command",
      });
      await expect(copy).toBeFocused();
      await copy.click();
      await expect(copy).toHaveCount(0);
      const result = await page.evaluate(async () => JSON.parse(await navigator.clipboard.readText()));
      const feature = baseDocument.features[1];
      expect(result).toEqual({ command: "add_feature", feature: { ...feature, id: expect.any(String) } });
      expect(result.feature.id).not.toBe(feature.id);
      await expect(row).toBeFocused();
      expect(await page.evaluate(() => (window as any).__m2.project().currentRevision)).toBe(id);
    });
  }
}
