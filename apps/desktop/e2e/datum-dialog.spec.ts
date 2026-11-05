import { expect, test } from "@playwright/test";

const kinds = ["datumPlane", "datumAxis", "datumPoint"] as const;

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`datum creation sends one feature per command (${locale}, ${theme})`, async ({ page }, info) => {
      const labels = locale === "ru"
        ? { titles: ["Опорная плоскость", "Опорная ось", "Опорная точка"], origin: "Начало опорного элемента (мм)", normal: "Нормаль плоскости", build: "Построить" }
        : { titles: ["Datum plane", "Datum axis", "Datum point"], origin: "Datum origin (mm)", normal: "Plane normal", build: "Build" };
      await page.addInitScript((value) => localStorage.setItem("forma.locale", value), locale);
      await page.goto("/");
      const requests: unknown[] = [];
      await page.exposeFunction("recordDatum", (command: string, args: unknown) => {
        requests.push({ command, args });
      });
      await page.evaluate(async (theme) => {
        const React = await import("/node_modules/.vite/deps/react.js");
        const ReactDOM = await import("/node_modules/.vite/deps/react-dom_client.js");
        const { DatumDialog } = await import("/src/features/add-menu/datum/DatumDialog.tsx");
        const { captureDatum } = await import("/src/features/add-menu/datum/datumModel.ts");
        const { applyTheme } = await import("/src/lib/theme.ts");
        const { newProject } = await import("/src/stores/workspace.ts");
        const project = newProject("Datum fixture", "blank", "mm", "codex");
        const runtime = window as unknown as {
          __TAURI_INTERNALS__: { invoke: (command: string, args: unknown) => Promise<unknown> };
          recordDatum: (command: string, args: unknown) => Promise<void>;
          showDatum: (kind: string, anchored?: boolean) => void;
        };
        runtime.__TAURI_INTERNALS__ = {
          invoke: async (command, args) => {
            await runtime.recordDatum(command, args);
            return project;
          },
        };
        applyTheme(theme);
        const root = ReactDOM.default.createRoot(document.body.appendChild(document.createElement("div")));
        runtime.showDatum = (kind, anchored) => root.render(React.default.createElement(DatumDialog, {
          key: kind, kind, disabled: false, close: () => root.render(null),
          sources: anchored ? [{ label: "Fixture plane", source: { type: "plane", featureId: "base-plane" } }] : [],
          onApply: async (operation: Parameters<typeof captureDatum>[1]) => {
            await captureDatum(project, operation).invoke();
            root.render(null);
          },
        }));
      }, theme);
      for (const [index, kind] of kinds.entries()) {
        await page.evaluate((kind) => (window as unknown as { showDatum: (kind: string) => void }).showDatum(kind), kind);
        const dialog = page.getByRole("dialog", { name: labels.titles[index] });
        const build = dialog.getByRole("button", { name: labels.build, exact: true });
        await dialog.getByLabel(`${labels.origin} · X`).fill("2,5");
        if (kind === "datumPlane") {
          const normal = dialog.getByLabel(`${labels.normal} · Z`);
          await normal.fill("0");
          await expect(build).toBeDisabled();
          await normal.fill("1");
          await dialog.screenshot({ path: info.outputPath("datum-plane.png") });
        }
        await build.click();
        await expect(dialog).toHaveCount(0);
        expect(requests).toHaveLength(index + 1);
        expect(requests[index]).toMatchObject({
          command: "apply_ir_commands",
          args: { expectedRevision: null, commands: [
            { command: "add_feature", feature: { operation: { type: kind, originMm: [2.5, 0, 0] } } },
          ] },
        });
      }
      await page.evaluate(() => (window as unknown as {
        showDatum: (kind: string, anchored: boolean) => void;
      }).showDatum("datumPoint", true));
      const dialog = page.getByRole("dialog", { name: labels.titles[2] });
      await dialog.getByRole("combobox").click();
      await page.getByRole("option", { name: "Fixture plane", exact: true }).click();
      await dialog.getByLabel(locale === "ru" ? "Смещение опорного элемента (мм)" : "Datum offset (mm)").fill("-2,5");
      await dialog.getByRole("button", { name: labels.build, exact: true }).click();
      await expect(dialog).toHaveCount(0);
      expect(requests[3]).toMatchObject({
        command: "apply_ir_commands",
        args: { commands: [{ command: "add_feature", feature: { operation: {
          type: "datumOnReference", geometry: { type: "point" },
          anchor: { source: { type: "plane", featureId: "base-plane" }, offsetMm: -2.5, angleDeg: 0 },
        } } }] },
      });
    });
  }
}
