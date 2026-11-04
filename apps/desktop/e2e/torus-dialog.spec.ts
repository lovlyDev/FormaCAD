import { expect, test } from "@playwright/test";

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`torus dimensions and command batch (${locale}, ${theme})`, async ({ page }, info) => {
      const labels = locale === "ru"
        ? { title: "Добавить тор", major: "Большой радиус", tube: "Радиус трубки", build: "Построить" }
        : { title: "Add torus", major: "Major radius", tube: "Tube radius", build: "Build" };
      await page.addInitScript((value) => localStorage.setItem("forma.locale", value), locale);
      await page.goto("/");
      const requests: unknown[] = [];
      const expectedRevision = theme === "light" ? null : "baseline";
      await page.exposeFunction("recordTorusCommand", (command: string, args: unknown) => {
        requests.push({ command, args });
      });
      await page.evaluate(async ({ theme, expectedRevision }) => {
        const React = await import("/node_modules/.vite/deps/react.js");
        const ReactDOM = await import("/node_modules/.vite/deps/react-dom_client.js");
        const { TorusDialog } = await import("/src/features/add-menu/torus/TorusDialog.tsx");
        const { captureTorus } = await import("/src/features/add-menu/torus/torusCommands.ts");
        const { applyTheme } = await import("/src/lib/theme.ts");
        const { newProject } = await import("/src/stores/workspace.ts");
        const project = newProject("Torus fixture", "blank", "mm", "codex");
        project.currentRevision = expectedRevision;
        const tauri = window as unknown as {
          __TAURI_INTERNALS__: { invoke: (command: string, args: unknown) => Promise<unknown> };
          recordTorusCommand: (command: string, args: unknown) => Promise<void>;
        };
        tauri.__TAURI_INTERNALS__ = {
          invoke: async (command, args) => {
            await tauri.recordTorusCommand(command, args);
            return project;
          },
        };
        applyTheme(theme);
        const root = ReactDOM.default.createRoot(document.body.appendChild(document.createElement("div")));
        root.render(React.default.createElement(TorusDialog, {
          close: () => root.unmount(),
          disabled: false,
          onApply: (major: number, minor: number) => {
            void captureTorus(project, expectedRevision, major, minor).invoke();
          },
        }));
      }, { theme, expectedRevision });
      const dialog = page.getByRole("dialog", { name: labels.title });
      const build = dialog.getByRole("button", { name: labels.build, exact: true });
      await dialog.getByLabel(labels.tube).fill("10");
      await expect(build).toBeDisabled();
      await expect(dialog.getByRole("status")).toBeVisible();
      expect(requests).toHaveLength(0);
      await dialog.getByLabel(labels.major).fill("12,5");
      await dialog.getByLabel(labels.tube).fill("3");
      await expect(build).toBeEnabled();
      await dialog.screenshot({ path: info.outputPath("torus-dialog.png") });
      await build.click();
      await expect.poll(() => requests.length).toBe(1);
      expect(requests[0]).toMatchObject({
        command: "apply_ir_commands",
        args: {
          expectedRevision,
          commands: [
            { command: "add_feature", feature: {
              operation: {
                type: "torus",
                majorRadius: { kind: "literal", mm: 12.5 },
                minorRadius: { kind: "literal", mm: 3 },
              },
            } },
            { command: "add_body" },
          ],
        },
      });
    });
  }
}
