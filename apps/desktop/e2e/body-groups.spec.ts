import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import { baseDocument, viewportTransformWorker } from "./viewportTransformFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`body groups (${locale}, ${theme})`, async ({ page }) => {
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
      const panel = page.getByRole("region", { name: ru ? "Группы тел" : "Body groups" });
      const approve = () => page.getByRole("button", { name: ru ? "Разрешить один раз" : "Allow once", exact: true }).click();
      await panel.getByRole("textbox").fill("Fixture group");
      await panel.getByRole("button", { name: ru ? "Создать группу" : "Create group" }).click();
      await approve();
      const group = panel.getByRole("button", { name: "Fixture group", exact: true });
      await expect(group).toBeDisabled();
      await page.getByRole("button", { name: "Body", exact: true }).click();
      await panel.getByRole("button", { name: ru ? "Добавить выделенное в Fixture group" : "Add selection to Fixture group" }).click();
      await approve();
      await expect(group).toBeEnabled();
      await page.getByRole("button", { name: ru ? "Снять выделение" : "Clear selection", exact: true }).click();
      await expect.poll(() => page.evaluate(() => (window as any).__m2.selection())).toEqual([]);
      await group.click();
      await expect.poll(() => page.evaluate(() => (window as any).__m2.selection())).toEqual(["body"]);
      await panel.getByRole("button", { name: ru ? "Убрать выделенное из Fixture group" : "Remove selection from Fixture group" }).click();
      await approve();
      await expect(group).toBeDisabled();
      await panel.getByRole("button", { name: ru ? "Удалить группу Fixture group" : "Delete group Fixture group" }).click();
      await approve();
      await expect(group).toHaveCount(0);
      await expect(page.getByRole("button", { name: "Body", exact: true })).toBeVisible();
    });
  }
}
