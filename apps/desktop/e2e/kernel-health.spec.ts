import { expect, test } from "@playwright/test";

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`unavailable kernel explanation ${locale}/${theme}`, async ({ page }, testInfo) => {
      await page.addInitScript(({ locale, theme }) => {
        localStorage.setItem("forma.locale", locale);
        localStorage.setItem("forma.theme", theme);
        const diagnostic = JSON.stringify({
          code: "NATIVE_KERNEL_UNAVAILABLE",
          message: "TKernel.dll loader failed at private-install-path",
        });
        Object.assign(window, {
          isTauri: true,
          __TAURI_INTERNALS__: {
            metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
            transformCallback: () => 0,
            unregisterCallback: () => {},
            invoke: async (command: string) => {
              if (command === "get_ui_preferences") return {};
              if (command === "detect_environment") {
                return [{ name: "OpenCascade kernel", available: false, detail: diagnostic }];
              }
              if (["list_projects", "interrupted_sessions", "permission_audit"].includes(command)) return [];
              return null;
            },
          },
        });
      }, { locale, theme });
      await page.goto("/");
      await page.getByRole("button", { name: locale === "ru" ? "Настройки" : "Settings", exact: true }).first().click();
      await page.getByRole("button", { name: locale === "ru" ? "CAD-окружение" : "CAD environment", exact: true }).click();
      const row = page.locator(".health-row");
      await expect(row).toContainText(locale === "ru" ? "CAD-ядро не удалось запустить" : "The CAD engine could not start");
      await expect(row).not.toContainText("private-install-path");
      await expect(row).not.toContainText('"message"');
      await expect(row).toContainText(locale === "ru" ? "Недоступно" : "Unavailable");
      await expect(page.getByText(locale === "ru"
        ? "OpenCascade выполняет импорт STEP и CAD-моделирование. Python и CadQuery нужны только для прежних сценариев."
        : "OpenCascade handles STEP import and CAD modeling. Python and CadQuery are optional for legacy workflows.", { exact: true })).toBeVisible();
      await expect(row.getByRole("status")).toBeVisible();
      await expect.poll(() => row.evaluate(element => {
        for (let node: Element | null = element; node; node = node.parentElement) {
          if (node.getAnimations().some(animation => animation.playState === "running" || animation.pending)) return false;
        }
        return true;
      })).toBe(true);
      await page.screenshot({ path: testInfo.outputPath(`kernel-health-${locale}-${theme}.png`) });
    });
  }
}
