import { expect, test } from "@playwright/test";

for (const locale of ["ru", "en"] as const) test(`project transfer review ${locale}: themes, repeated motion, redaction and manifest capture`, async ({ page }, info) => {
  page.on("pageerror", error => console.error(error.message));
  await page.addInitScript(locale => localStorage.setItem("forma.locale", locale), locale);
  await page.goto("/");
  await page.evaluate(async () => {
    const React = await import("/node_modules/.vite/deps/react.js"), ReactDOM = await import("/node_modules/.vite/deps/react-dom_client.js");
    const { TooltipProvider } = await import("/src/components/ui.tsx");
    const { ImportProjectFolderButton } = await import("/src/features/project-folder/ImportProjectFolderButton.tsx");
    const { ExportProjectBundleControl } = await import("/src/features/project-bundle/ExportProjectBundleControl.tsx");
    const project = { id: "reviewed_project", name: "Checked project" };
    window.__transferCalls = []; window.__transferExports = [];
    window.__TAURI_INTERNALS__ = { invoke: async (command: string, args: unknown) => {
      window.__transferCalls.push({ command, args });
      if (command === "inspect_project_folder") return { path: "C:\\Projects\\checked", projectId: project.id, name: project.name, currentRevision: "rev", revisionCount: 3, manifestSha256: "a".repeat(64), identityExists: true };
      if (command === "import_project_folder") return project;
      throw new Error(command);
    } };
    const host = document.body.appendChild(document.createElement("div"));
    host.id = "transfer-test";
    const root = ReactDOM.default.createRoot(host);
    root.render(React.default.createElement(TooltipProvider, null,
      React.default.createElement(ImportProjectFolderButton, { onImported: () => {}, onError: (error: string) => { throw new Error(error); } }),
      React.default.createElement(ExportProjectBundleControl, { project, onExport: (_: unknown, redact: boolean) => window.__transferExports.push(redact) })));
  });
  const labels = locale === "ru" ? { import: "Импортировать папку проекта", accept: "Импортировать проект", cancel: "Отмена", export: "Экспортировать пакет проекта", exportAction: "Экспорт", redact: "Исключить переписку и текст действий" }
    : { import: "Import project folder", accept: "Import project", cancel: "Cancel", export: "Export project bundle", exportAction: "Export", redact: "Exclude conversation and activity text" };
  await expect(page.locator("#transfer-test").getByRole("button", { name: labels.import, exact: true })).toBeVisible({ timeout: 10000 });
  for (const theme of ["dark", "light"]) {
    await page.evaluate(theme => document.documentElement.dataset.theme = theme, theme);
    for (let repeat = 0; repeat < 2; repeat++) {
      await page.getByRole("button", { name: labels.import, exact: true }).click();
      const dialog = page.getByRole("dialog"); await expect(dialog).toBeVisible();
      await expect(dialog).toContainText("Checked project"); await expect(dialog.getByRole("checkbox")).toBeChecked();
      await expect(dialog.getByRole("checkbox")).toBeDisabled();
      expect(await dialog.evaluate(element => getComputedStyle(element).animationDuration)).toBe("0.2s");
      const box = await dialog.boundingBox(); expect(box!.width).toBeLessThanOrEqual(680);
      await dialog.getByRole("button", { name: labels.cancel, exact: true }).click(); await expect(dialog).toBeHidden();
    }
    await page.getByRole("button", { name: labels.export, exact: true }).click();
    const dialog = page.getByRole("dialog"); await expect(dialog).toBeVisible();
    const checkbox = dialog.getByRole("checkbox", { name: labels.redact });
    if (!await checkbox.isChecked()) await checkbox.click();
    await page.screenshot({ path: info.outputPath(`transfer-${locale}-${theme}.png`) });
    await dialog.getByRole("button", { name: labels.exportAction, exact: true }).click(); await expect(dialog).toBeHidden();
  }
  expect(await page.evaluate(() => window.__transferExports)).toEqual([true, true]);
  await page.getByRole("button", { name: labels.import, exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: labels.accept, exact: true }).click();
  await expect.poll(() => page.evaluate(() => window.__transferCalls.filter((call: { command: string }) => call.command === "import_project_folder").length)).toBe(1);
  expect(await page.evaluate(() => window.__transferCalls.find((call: { command: string }) => call.command === "import_project_folder").args)).toEqual({ path: "C:\\Projects\\checked", expectedManifestSha256: "a".repeat(64), saveCopy: true });
});
