import { expect, test } from "@playwright/test";

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`settings defer environment processes ${locale}/${theme}`, async ({ page }) => {
      await page.addInitScript(({ locale, theme }) => {
        localStorage.setItem("forma.locale", locale);
        localStorage.setItem("forma.theme", theme);
        localStorage.setItem("forma.ui.settingsPage", JSON.stringify("Appearance"));
        const calls: string[] = [];
        Object.assign(window, {
          isTauri: true,
          profileCheckCalls: calls,
          __TAURI_INTERNALS__: {
            metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
            transformCallback: () => 0,
            unregisterCallback: () => {},
            invoke: async (command: string) => {
              calls.push(command);
              if (command === "get_ui_preferences") return {};
              if (command === "detect_environment") return [{ name: "OpenCascade kernel", available: true, detail: "ready" }];
              if (["list_projects", "interrupted_sessions", "permission_audit"].includes(command)) return [];
              return null;
            },
          },
        });
      }, { locale, theme });
      const environmentCalls = () => page.evaluate(() =>
        (window as unknown as { profileCheckCalls: string[] }).profileCheckCalls.filter(value => value === "detect_environment").length);
      await page.goto("/");
      await page.getByRole("button", { name: locale === "ru" ? "Настройки" : "Settings", exact: true }).first().click();
      const dialog = page.getByRole("dialog");
      await expect(dialog).toBeVisible();
      await expect(dialog.getByRole("button", { name: locale === "ru" ? "Внешний вид" : "Appearance", exact: true })).toBeVisible();
      expect(await environmentCalls()).toBe(0);
      await dialog.getByRole("button", { name: locale === "ru" ? "Конфиденциальность" : "Privacy", exact: true }).click();
      expect(await environmentCalls()).toBe(0);
      await dialog.getByRole("button", { name: locale === "ru" ? "Разрешения" : "Permissions", exact: true }).click();
      await expect.poll(() => page.evaluate(() =>
        (window as unknown as { profileCheckCalls: string[] }).profileCheckCalls.includes("permission_audit"))).toBe(true);
      expect(await environmentCalls()).toBe(0);
      await dialog.getByRole("button", { name: locale === "ru" ? "CAD-окружение" : "CAD environment", exact: true }).click();
      await expect.poll(environmentCalls).toBe(1);
      await expect(dialog.locator(".health-row")).toContainText(locale === "ru" ? "Ядро OpenCascade" : "OpenCascade kernel");
    });
  }
}
