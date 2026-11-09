import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import { baseDocument, viewportTransformWorker } from "./viewportTransformFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`project activity is separate from errors (${locale}, ${theme})`, async ({ page }) => {
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
      const ru = locale === "ru";
      const activity = ru ? "Активность" : "Activity";
      const logs = ru ? "Журналы" : "Logs";
      const event = ru ? "Ревизия модели сохранена" : "Model revision saved";
      await page.getByRole("button", { name: activity, exact: true }).click();
      const activityPanel = page.getByRole("region", { name: activity, exact: true });
      await expect(activityPanel.getByText(event, { exact: true })).toBeVisible();
      await expect(activityPanel.getByText("Baseline", { exact: true })).toBeVisible();
      await page.getByRole("button", { name: logs, exact: true }).click();
      const logsPanel = page.getByRole("region", { name: logs, exact: true });
      await expect(logsPanel.getByText(ru ? "Ошибок не записано" : "No recorded errors")).toBeVisible();
      await expect(logsPanel.getByText(event, { exact: true })).toHaveCount(0);
    });
  }
}
