import { expect, test } from "@playwright/test";
import { formatThemeFile } from "../src/features/theme-editor/themePalette";
import { defaultPalettes } from "../src/features/theme-editor/themePreferences";

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`custom palette import, apply and reset (${locale}, ${theme})`, async ({ page }) => {
      const ru = locale === "ru";
      await page.goto("/");
      const id = await page.evaluate(({ locale, theme }) => {
        const id = crypto.randomUUID();
        const revision = crypto.randomUUID();
        const now = new Date().toISOString();
        localStorage.setItem("forma.locale", locale);
        localStorage.setItem("forma.theme", theme);
        localStorage.setItem("forma.projects.v1", JSON.stringify([{
          id, name: "Theme check", schemaVersion: 1, units: "mm", agent: "codex",
          pinned: false, createdAt: now, updatedAt: now, currentRevision: revision,
          messages: [], files: [], exports: [], revisions: [{
            id: revision, parent: null, createdAt: now, prompt: "Model",
            parameters: { kind: "box", width: 60, depth: 40, height: 10, thickness: 2, holeDiameter: 0, holes: 0 },
          }],
        }]));
        return id;
      }, { locale, theme });
      await page.goto(`/#/project/${id}`);
      await page.reload();
      const canvas = page.locator(".viewer-area canvas");
      await expect(canvas).toHaveAttribute("data-background", defaultPalettes[theme].viewport);
      const open = async () => {
        await page.getByRole("button", { name: ru ? "Настройки" : "Settings", exact: true }).first().click();
        await page.getByRole("button", { name: ru ? "Внешний вид" : "Appearance", exact: true }).click();
        await page.getByText(ru ? "Редактор тем" : "Theme editor", { exact: true }).click();
      };
      await open();
      const base = defaultPalettes[theme];
      const color = theme === "dark" ? "#293943" : "#d5e8f2";
      const upload = (contents: string) => page.getByLabel(ru ? "Импортировать тему из JSON" : "Import theme JSON").setInputFiles({
        name: "palette.json", mimeType: "application/json", buffer: Buffer.from(contents),
      });
      const apply = page.getByRole("button", { name: ru ? "Применить свою тему" : "Apply custom theme" });
      await upload(formatThemeFile({ ...base, text: base.surface }));
      await expect(apply).toBeDisabled();
      await upload(formatThemeFile({ ...base, viewport: color }));
      await expect(apply).toBeEnabled();
      await apply.click();
      await expect(canvas).toHaveAttribute("data-background", color);
      const download = page.waitForEvent("download");
      await page.getByRole("button", { name: ru ? "Экспортировать тему в JSON" : "Export theme JSON" }).click();
      expect((await download).suggestedFilename()).toBe("forma-theme.json");
      await page.reload();
      await expect(canvas).toHaveAttribute("data-background", color);
      await open();
      await page.getByRole("button", { name: ru ? "Восстановить тему по умолчанию" : "Restore default theme" }).click();
      await expect(canvas).toHaveAttribute("data-background", base.viewport);
    });
  }
}
