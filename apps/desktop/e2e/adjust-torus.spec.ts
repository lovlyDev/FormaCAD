import { expect, test } from "@playwright/test";

const revision = "fc7fcb6c-ef53-4a7e-b7eb-613a981b1b93";
const amended = "eaf64d34-0401-4acd-a827-6b6b646797ee";
for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`adjust latest torus radii (${locale}, ${theme})`, async ({ page }, info) => {
      await page.addInitScript((value) => localStorage.setItem("forma.locale", value), locale);
      await page.goto("/");
      await expect(page.getByRole("menubar")).toBeVisible();
      const requests: unknown[] = [];
      await page.exposeFunction("recordTorusReplacement", (args: unknown) => { requests.push(args); });
      await page.evaluate(async ({ theme, revision, amended }) => {
        const { useTransformViewport } = await import("/src/features/transform/viewport/transformStore.ts");
        const { applyTheme } = await import("/src/lib/theme.ts");
        const { newProject, useWorkspace } = await import("/src/stores/workspace.ts");
        const project = newProject("Torus fixture", "blank", "mm", "codex");
        project.currentRevision = revision;
        useWorkspace.getState().setProject(project);
        const tauri = window as unknown as {
          __TAURI_INTERNALS__: { invoke: (command: string, args: unknown) => Promise<unknown> };
          recordTorusReplacement: (args: unknown) => Promise<void>;
        };
        tauri.__TAURI_INTERNALS__ = {
          invoke: async (command, args) => {
            if (command === "get_confirmation_settings") return { mode: "none", overrides: {} };
            if (command === "last_torus")
              return { expectedRevision: revision, expectedGeneration: 1, transactionId: revision,
                radii: { majorRadiusMm: 10, minorRadiusMm: 3 } };
            if (command === "amend_last_torus") {
              await tauri.recordTorusReplacement(args);
              return { ...project, currentRevision: amended };
            }
          },
        };
        applyTheme(theme);
        useTransformViewport.getState().setAdjustOpen(true);
      }, { theme, revision, amended });
      const title = locale === "ru" ? "Изменить тор" : "Adjust torus";
      const dialog = page.getByRole("dialog", { name: title });
      const major = dialog.getByLabel(locale === "ru" ? "Большой радиус" : "Major radius");
      await expect(major).toHaveValue("10");
      await major.fill("12,5");
      await dialog.screenshot({ path: info.outputPath("adjust-torus.png") });
      await dialog.getByRole("button", { name: locale === "ru" ? "Применить" : "Apply", exact: true }).click();
      await page.getByRole("button", { name: locale === "ru" ? "Разрешить один раз" : "Allow once", exact: true }).click();
      await expect(dialog).toHaveCount(0);
      expect(requests).toHaveLength(1);
      expect(requests[0]).toMatchObject({
        replacement: { expectedRevision: revision, expectedGeneration: 1, transactionId: revision,
          radii: { majorRadiusMm: 12.5, minorRadiusMm: 3 } },
      });
    });
  }
}
