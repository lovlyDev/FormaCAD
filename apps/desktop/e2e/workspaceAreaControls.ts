import { expect, type Page } from "@playwright/test";
/** Use the user-visible editor selector before exercising contextual measurements. */
export async function openPropertiesEditor(page: Page, locale: "ru" | "en") {
  const editor = page.locator('[data-editor="properties"]');
  if (await editor.isVisible()) return;
  await page.locator('[data-editor="ai"]').getByRole("combobox").click();
  await page
    .getByRole("option", {
      name: locale === "ru" ? "Свойства" : "Properties",
      exact: true,
    })
    .click();
  await expect(editor).toBeVisible();
}
