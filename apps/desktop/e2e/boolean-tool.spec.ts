import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import { baseDocument, viewportTransformWorker } from "./viewportTransformFixture";
import { buildPreviewPacket } from "./viewportPreviewFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");
const literal = (mm: number) => ({ kind: "literal", mm });
const pair = {
  ...baseDocument,
  features: [...baseDocument.features,
    { id: "tool_profile", name: "Tool profile", operation: { type: "rectangle", width: literal(10), depth: literal(10) } },
    { id: "tool_pad", name: "Tool pad", operation: { type: "extrude", sketchId: "tool_profile", distance: literal(10) } },
    { id: "tool_offset", name: "Tool offset", operation: { type: "translate", bodyFeatureId: "tool_pad", offsetMm: [20, 0, 0] } },
  ],
  bodies: [...baseDocument.bodies, { id: "tool", name: "Tool", sourceFeatureId: "tool_offset" }],
};

for (const locale of ["ru", "en"] as const) {
  for (const theme of ["dark", "light"] as const) {
    test(`Boolean preserves its tool body (${locale}, ${theme})`, async ({ page }) => {
      const ru = locale === "ru";
      const id = randomUUID();
      const mode = locale === "ru" ? "cut" : theme === "light" ? "union" : "intersect";
      await page.exposeFunction("__m2Build", (document: unknown, id: string) => worker!.build(document, id));
      await page.exposeFunction("__m3Preview", (source: string) => buildPreviewPacket(worker!, source));
      await page.route(/\/src\/main\.tsx(?:\?.*)?$/, (route) => route.fulfill({
        contentType: "application/javascript", body: "export {};",
      }));
      await page.addInitScript((value) => localStorage.setItem("forma.locale", value), locale);
      await page.goto("/");
      await page.evaluate(async ({ base, theme }) => {
        const { mountViewportTransformHarness } = await import("/e2e/viewportTransformHarness.tsx");
        await mountViewportTransformHarness(base);
        (window as any).__m2.theme(theme);
      }, { base: { ...worker!.build(pair, id), revisionId: id }, theme });
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase", "ready");
      const title = ru ? "Булева операция" : "Boolean operation";
      await page.getByRole("menuitem", { name: ru ? "Инструменты" : "Tools", exact: true }).click();
      await page.getByRole("menuitem", { name: title, exact: true }).click();
      const dialog = page.getByRole("dialog", { name: title, exact: true });
      await expect(dialog.getByLabel(ru ? "Целевое тело" : "Target body")).toHaveValue("body");
      await expect(dialog.getByLabel(ru ? "Тело-инструмент" : "Tool body")).toHaveValue("tool");
      await dialog.getByLabel(ru ? "Режим булевой операции" : "Boolean mode").selectOption(mode);
      await expect(dialog.getByText(ru ? "Геометрия черновика корректна" : "Draft geometry is valid")).toBeVisible();
      await dialog.getByRole("button", { name: ru ? "Построить" : "Build", exact: true }).click();
      await page.getByRole("button", { name: ru ? "Разрешить один раз" : "Allow once", exact: true }).click();
      const revision = () => page.evaluate(() => (window as any).__m2.project().currentRevision);
      await expect.poll(revision).not.toBe(id);
      const committed = (await revision()) as string;
      expect(worker!.measure(committed, "body", { kind: "bodyMetrics" }).result.volumeMm3)
        .toBeCloseTo({ cut: 7500, union: 8500, intersect: 500 }[mode], 5);
      expect(worker!.measure(committed, "tool", { kind: "bodyMetrics" }).result.volumeMm3).toBeCloseTo(1000, 5);
      const batches = await page.evaluate(() => (window as any).__m2.calls
        .filter((call: any) => call.command === "apply_ir_commands").map((call: any) => call.args.commands));
      expect(batches).toMatchObject([[
        { command: "add_feature", feature: { operation: { type: "boolean", mode, leftFeatureId: "pad", rightFeatureId: "tool_offset" } } },
        { command: "set_body_source", bodyId: "body" },
      ]]);
    });
  }
}
