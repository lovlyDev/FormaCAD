import { selectValue } from "./customSelect";
import { expect, test } from "@playwright/test";

for (const locale of ["ru", "en"] as const) {
  test(`construction line edits preserve profile, constraints and scene in ${locale} and both themes`, async ({ page }, testInfo) => {
    await page.addInitScript((locale) => localStorage.setItem("forma.locale", locale), locale);
    await page.goto("/");
    const id = await page.evaluate(async () => {
      const { createStarterSketch } = await import("/src/lib/sketchDocument.ts");
      const id = crypto.randomUUID(), rid = crypto.randomUUID(), now = new Date().toISOString();
      localStorage.setItem("forma.projects.v1", JSON.stringify([{
        schemaVersion: 1, id, name: "Construction check", units: "mm", agent: "codex", pinned: false,
        createdAt: now, updatedAt: now, currentRevision: rid, messages: [], files: [], exports: [],
        revisions: [{ id: rid, parent: null, createdAt: now, prompt: "Sketch",
          program: createStarterSketch(), parameters: { kind: "box", width: 40, depth: 20, height: 10, thickness: 2, holeDiameter: 0, holes: 0 } }],
      }]));
      return id;
    });
    await page.goto(`/#/project/${id}`);
    await page.reload();
    const labels = locale === "ru" ? {
      edit: "Изменить параметры модели", sketch: "Редактировать 2D-эскиз", start: "Начальная точка вспомогательной линии",
      end: "Конечная точка вспомогательной линии", add: "Добавить вспомогательную линию", remove: "Удалить вспомогательную линию",
      length: "Длина", mm: "Длина в мм", source: "Исходник CAD-модели", settings: "Настройки", appearance: "Внешний вид",
      light: "Светлая", dark: "Тёмная", close: "Закрыть диалог",
    } : {
      edit: "Edit model parameters", sketch: "Edit 2D sketch", start: "Construction start point",
      end: "Construction end point", add: "Add construction line", remove: "Remove construction line",
      length: "Length", mm: "Length in mm", source: "CAD source", settings: "Settings", appearance: "Appearance",
      light: "Light", dark: "Dark", close: "Close dialog",
    };
    await expect(page.locator("canvas")).toBeVisible();
    await page.getByRole("button", { name: labels.edit, exact: true }).click();
    await page.getByText(labels.sketch, { exact: true }).click();
    await selectValue(page,page.getByLabel(labels.end, { exact: true }),"point_a");
    await expect(page.getByRole("button", { name: labels.add, exact: true })).toBeDisabled();
    await selectValue(page,page.getByLabel(labels.end, { exact: true }),"point_c");
    await page.getByRole("button", { name: labels.add, exact: true }).click();
    await expect(page.locator(".sketch-editor-construction")).toHaveCount(1);
    await expect(page.locator(".sketch-editor-construction")).toHaveCSS("stroke-dasharray", "7px, 4px");
    const guideRow = page.locator(".sketch-editor-lines > div").filter({ hasText: "construction_1" });
    await guideRow.getByRole("button", { name: labels.length, exact: true }).click();
    await page.getByLabel(labels.mm, { exact: true }).fill("50");
    const readSource = async () => JSON.parse(await page.getByLabel(labels.source, { exact: true }).inputValue());
    const source = await readSource();
    expect(source.features[0].operation.lines).toHaveLength(5);
    expect(source.features[0].operation.lines[4]).toMatchObject({ id: "construction_1", construction: true, startPointId: "point_a", endPointId: "point_c" });
    expect(source.features[0].operation.constraints.at(-1).distance.mm).toBe(50);
    expect(source.features[1].operation.sketchId).toBe("sketch_1");
    await page.locator(".sketch-editor-construction-tools").screenshot({ path: testInfo.outputPath(`construction-${locale}-dark.png`) });
    await expect.poll(async () => {
      const canvas = page.locator("canvas");
      const before = await canvas.getAttribute("data-camera");
      await page.waitForTimeout(600);
      return before === await canvas.getAttribute("data-camera");
    }).toBe(true);
    const camera = await page.locator("canvas").getAttribute("data-camera");
    const darkStroke = await page.locator(".sketch-editor-construction").evaluate((node) => getComputedStyle(node).stroke);
    const setTheme = async (theme: "light" | "dark") => page.evaluate(async (theme) => {
      const { applyTheme } = await import("/src/lib/theme.ts");
      applyTheme(theme);
    }, theme);
    await setTheme("light");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    await expect(page.locator(".sketch-editor-construction")).not.toHaveCSS("stroke", darkStroke);
    expect(await readSource()).toEqual(source);
    expect(await page.locator("canvas").getAttribute("data-camera")).toEqual(camera);
    await page.locator(".sketch-editor-construction-tools").screenshot({ path: testInfo.outputPath(`construction-${locale}-light.png`) });
    await setTheme("dark");
    await expect(page.locator(".sketch-editor-construction")).toHaveCSS("stroke", darkStroke);
    expect(await readSource()).toEqual(source);
    await page.getByRole("button", { name: `${labels.remove} construction_1`, exact: true }).click();
    await expect(page.locator(".sketch-editor-construction")).toHaveCount(0);
    const removed = await readSource();
    expect(removed.features[0].operation.lines).toHaveLength(4);
    expect(removed.features[0].operation.constraints).toHaveLength(0);
    expect(removed.features[0].operation.points).toEqual(source.features[0].operation.points);
  });
}
