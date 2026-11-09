import { expect, test } from "@playwright/test";

const revision = "fc7fcb6c-ef53-4a7e-b7eb-613a981b1b93";
const amended = "eaf64d34-0401-4acd-a827-6b6b646797ee";
for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`adjust latest datum coordinates (${locale}, ${theme})`, async ({ page }, info) => {
      page.on("pageerror", (error) => console.error(error));
      await page.addInitScript((value) => localStorage.setItem("forma.locale", value), locale);
      await page.goto("/");
      await expect(page.getByRole("menubar")).toBeVisible();
      const requests: unknown[] = [];
      await page.exposeFunction("recordDatumReplacement", (args: unknown) => { requests.push(args); });
      await page.evaluate(async ({ theme, revision, amended }) => {
        const { applyTheme } = await import("/src/lib/theme.ts");
        const { newProject, useWorkspace } = await import("/src/stores/workspace.ts");
        const project = newProject("Datum fixture", "blank", "mm", "codex");
        project.currentRevision = revision;
        const operation = { type: "datumPoint", originMm: [2, 0, 0] };
        project.revisions = [{
          id: revision, parent: null, createdAt: project.createdAt, prompt: "Datum fixture",
          parameters: { kind: "blank", width: 1, depth: 1, height: 1, thickness: 1, holeDiameter: 0, holes: 0 },
          program: JSON.stringify({ schemaVersion: 2, revisionId: revision, parameters: [], bodies: [],
            features: [{ id: "datum", name: "Datum fixture", operation }] }),
        }];
        useWorkspace.getState().setProject(project);
        const tauri = window as unknown as {
          __TAURI_INTERNALS__: { invoke: (command: string, args: unknown) => Promise<unknown> };
          recordDatumReplacement: (args: unknown) => Promise<void>;
        };
        tauri.__TAURI_INTERNALS__ = {
          invoke: async (command, args) => {
            if (command === "get_confirmation_settings") return { mode: "none", overrides: {} };
            if (command === "last_datum")
              return { expectedRevision: revision, expectedGeneration: 1, transactionId: revision,
                operation };
            if (command === "amend_last_datum") {
              await tauri.recordDatumReplacement(args);
              return { ...project, currentRevision: amended };
            }
          },
        };
        applyTheme(theme);
      }, { theme, revision, amended });
      await expect(page.locator(".viewer-area canvas").first()).toBeVisible();
      await page.evaluate(async () => {
        const { useTransformViewport } = await import("/src/features/transform/viewport/transformStore.ts");
        useTransformViewport.getState().setAdjustOpen(true);
      });
      const title = locale === "ru" ? "Опорная точка" : "Datum point";
      const dialog = page.getByRole("dialog", { name: title });
      const major = dialog.getByLabel(locale === "ru" ? "Начало опорного элемента (мм) · X" : "Datum origin (mm) · X");
      await expect(major).toHaveValue("2");
      await major.fill("12,5");
      await dialog.screenshot({ path: info.outputPath("adjust-datum.png") });
      await dialog.getByRole("button", { name: locale === "ru" ? "Применить" : "Apply", exact: true }).click();
      await page.getByRole("button", { name: locale === "ru" ? "Разрешить один раз" : "Allow once", exact: true }).click();
      await expect(dialog).toHaveCount(0);
      expect(requests).toHaveLength(1);
      expect(requests[0]).toMatchObject({
        replacement: { expectedRevision: revision, expectedGeneration: 1, transactionId: revision,
          operation: { type: "datumPoint", originMm: [12.5, 0, 0] } },
      });
    });
  }
}
