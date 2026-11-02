import { expect, test, type Page } from "@playwright/test";
import {
  baseDocument,
  viewportTransformWorker,
} from "./viewportTransformFixture";

const worker = viewportTransformWorker();
test.skip(!worker, "Requires the Windows native CAD worker");

const close = (actual: number, expected: number) =>
  expect(Math.abs(actual - expected)).toBeLessThan(1e-3);

async function mount(page: Page, locale: "ru" | "en") {
  page.on("pageerror", (error) => console.log("APP ERROR", error.stack));
  await page.exposeFunction(
    "__m2Snap",
    (revisionId: string, bodyId: string, query: unknown) =>
      worker!.snap(revisionId, bodyId, query),
  );
  await page.exposeFunction(
    "__m2Build",
    (document: unknown, revisionId: string) =>
      worker!.build(document, revisionId),
  );
  await page.route(/\/src\/main\.tsx(?:\?.*)?$/, (route) =>
    route.fulfill({
      contentType: "application/javascript",
      body: "export {};",
    }),
  );
  await page.addInitScript(
    (locale) => localStorage.setItem("forma.locale", locale),
    locale,
  );
  await page.goto("/");
  const base = {
    ...worker!.build(baseDocument, "8d4e2f9c-0f6b-4a8e-9a43-3c1f5b7f0a11"),
    revisionId: "8d4e2f9c-0f6b-4a8e-9a43-3c1f5b7f0a11",
  };
  await page.evaluate(async (base) => {
    const { mountViewportTransformHarness } =
      await import("/e2e/viewportTransformHarness.tsx");
    await mountViewportTransformHarness(base);
  }, base);
  await expect(page.locator(".viewer-area")).toHaveAttribute(
    "data-scene-phase",
    "ready",
  );
  await expect
    .poll(() => page.evaluate(() => (window as any).__m2.bounds()))
    .not.toBeNull();
}
const bounds = (page: Page) =>
  page.evaluate(() => (window as any).__m2.bounds() as number[]);
const calls = (page: Page, command: string) =>
  page.evaluate(
    (command) =>
      (window as any).__m2.calls
        .filter((call: any) => call.command === command)
        .map((call: any) => call.args),
    command,
  );
async function selectBody(page: Page) {
  const point = await page.evaluate(() => (window as any).__m2.bodyScreen());
  await page.mouse.click(point.x, point.y);
  await expect
    .poll(() => page.evaluate(() => (window as any).__m2.selection()))
    .toEqual(["body"]);
  return point as { x: number; y: number };
}
async function decide(page: Page, label: string) {
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: label, exact: true }).click();
  await expect(dialog).toHaveCount(0);
}
async function waitRevision(page: Page, before: string) {
  await expect
    .poll(() =>
      page.evaluate(() => (window as any).__m2.project().currentRevision),
    )
    .not.toBe(before);
  await expect(page.locator(".viewer-area")).toHaveAttribute(
    "data-scene-phase",
    "ready",
  );
}

test("G X 10 mm, Esc, F9 and one Undo through the real worker (RU layout)", async ({
  page,
}) => {
  await mount(page, "ru");
  const point = await selectBody(page);
  const original = await bounds(page);

  // Esc after a visible preview restores the matrices and writes no revision.
  await page.keyboard.press("KeyG");
  await expect(page.locator(".transform-hud")).toContainText("Перемещение");
  await page.mouse.move(point.x + 80, point.y + 30, { steps: 4 });
  await expect
    .poll(async () => (await bounds(page))[0])
    .not.toBeCloseTo(original[0], 3);
  await page.keyboard.press("Escape");
  expect(await bounds(page)).toEqual(original);
  expect(await calls(page, "apply_ir_commands")).toHaveLength(0);
  await expect(page.locator(".transform-hud")).toHaveCount(0);

  // Denying the permission dialog restores the preview and writes nothing.
  await page.keyboard.press("KeyG");
  await page.keyboard.press("KeyY");
  await page.keyboard.type("5");
  await page.keyboard.press("Enter");
  await decide(page, "Отклонить");
  expect(await bounds(page)).toEqual(original);
  expect(await calls(page, "apply_ir_commands")).toHaveLength(0);

  // G X 10 Enter typed with Russian key characters: physical codes drive commands.
  const revision = await page.evaluate(
    () => (window as any).__m2.project().currentRevision,
  );
  // A user returns to the viewport with a click; the area then owns the keys.
  await page.locator('[data-editor="viewport3d"]').focus();
  await page.evaluate(() => {
    const target = document.activeElement ?? document.body;
    for (const [code, key] of [
      ["KeyG", "п"],
      ["KeyX", "ч"],
      ["Digit1", "1"],
      ["Digit0", "0"],
      ["Enter", "Enter"],
    ])
      target.dispatchEvent(
        new KeyboardEvent("keydown", {
          code,
          key,
          bubbles: true,
          cancelable: true,
        }),
      );
  });
  await decide(page, "Разрешить один раз");
  await waitRevision(page, revision);
  const [apply] = await calls(page, "apply_ir_commands");
  expect(apply.expectedRevision).toBe(revision);
  expect(apply.commands).toEqual([
    expect.objectContaining({
      command: "transform_body",
      bodyId: "body",
      matrix: [1, 0, 0, 10, 0, 1, 0, 0, 0, 0, 1, 0],
    }),
  ]);
  await expect
    .poll(async () => (await bounds(page))[0] - original[0])
    .toBeCloseTo(10, 3);
  const moved = await bounds(page);
  close(moved[3] - original[3], 10);
  close(moved[1], original[1]);
  close(moved[2], original[2]);

  // F9 replaces 10 mm by 20 mm relative to the original position.
  const afterMove = await page.evaluate(
    () => (window as any).__m2.project().currentRevision,
  );
  await page.keyboard.press("F9");
  const panel = page.getByRole("region", {
    name: "Параметры последней операции",
  });
  await expect(panel).toBeVisible();
  const x = panel.getByLabel("X, мм");
  await expect(x).toHaveValue("10");
  await x.fill("20");
  await panel.getByRole("button", { name: "Применить" }).click();
  await decide(page, "Разрешить один раз");
  await waitRevision(page, afterMove);
  const [amend] = await calls(page, "amend_last_transform");
  expect(amend.replacements).toEqual([
    { commandIndex: 0, matrix: [1, 0, 0, 20, 0, 1, 0, 0, 0, 0, 1, 0] },
  ]);
  await expect
    .poll(async () => (await bounds(page))[0] - original[0])
    .toBeCloseTo(20, 3);

  // One Ctrl+Z returns to the state before the original transform.
  await page.locator('[data-editor="viewport3d"]').focus();
  await page.keyboard.press("Control+KeyZ");
  await decide(page, "Разрешить один раз");
  await expect
    .poll(async () => (await calls(page, "undo_model")).length)
    .toBe(1);
  await expect
    .poll(async () => (await bounds(page))[0] - original[0])
    .toBeCloseTo(0, 3);
});

test("rotate preview, axis constraint, pies, visibility and themes (EN)", async ({
  page,
}) => {
  await mount(page, "en");
  const point = await selectBody(page);
  const original = await bounds(page);
  for (const theme of ["light", "dark"] as const) {
    await page.evaluate((theme) => (window as any).__m2.theme(theme), theme);
    await page.keyboard.press("KeyR");
    await page.keyboard.press("KeyZ");
    await expect(page.locator(".transform-hud")).toContainText("Rotate");
    await expect(page.locator(".transform-hud")).toContainText(
      "Axis Z (global)",
    );
    const colors = await page.locator(".transform-hud").evaluate((element) => {
      const style = getComputedStyle(element);
      return [style.color, style.backgroundColor];
    });
    expect(colors[0]).not.toBe(colors[1]);
    await page.mouse.move(point.x, point.y - 120, { steps: 4 });
    await page.keyboard.press("Escape");
    expect(await bounds(page)).toEqual(original);
  }
  // Shading pie by number, then visibility: hide and show all.
  await page.mouse.move(point.x, point.y);
  await page.keyboard.press("KeyZ");
  const pie = page.getByRole("menu", { name: "Shading" });
  await expect(pie).toBeVisible();
  await page.keyboard.press("Digit3");
  await expect(pie).toHaveCount(0);
  const visible = () => page.evaluate(() => (window as any).__m2.visible());
  await page.keyboard.press("KeyH");
  await expect.poll(visible).toBe(false);
  await page.keyboard.press("Alt+KeyH");
  await expect.poll(visible).toBe(true);
  // Standard views: front, back (Ctrl) and top look along the expected render axes.
  const direction = () =>
    page.evaluate(() => (window as any).__m2.viewDirection());
  for (const [key, expected] of [
    ["Numpad1", [0, 0, -1]],
    ["Control+Numpad1", [0, 0, 1]],
    ["Numpad7", [0, -1, 0]],
  ] as const) {
    await page.keyboard.press(key);
    await expect
      .poll(async () =>
        (await direction()).every(
          (value: number, index: number) =>
            Math.abs(value - expected[index]) < 1e-3,
        ),
      )
      .toBe(true);
  }
  expect(await calls(page, "apply_ir_commands")).toHaveLength(0);
});

test("modal preview frame rate", async ({ page }) => {
  await mount(page, "en");
  await selectBody(page);
  await page.keyboard.press("KeyG");
  const fps = await page.evaluate(() => (window as any).__m2.previewFps(2000));
  console.log(`M2 modal preview: ${fps.toFixed(1)} FPS (software WebGL)`);
  await page.keyboard.press("Escape");
  expect(fps).toBeGreaterThan(10);
});

test("vertex selection and ruler snap to exact OCCT vertices (RU)", async ({
  page,
}) => {
  await mount(page, "ru");
  await selectBody(page);
  const vertex = (role: string) =>
    page.evaluate((role) => (window as any).__m2.vertexScreen(role), role);
  // 40×20×10 box centred in XY: corner xmax/ymin/zmax is (20, −10, 10) mm.
  const corner = await vertex("box-vertex:xmax:ymin:zmax");
  await page.mouse.move(corner.x, corner.y);
  await page.keyboard.press("Digit4");
  await page.mouse.click(corner.x, corner.y);
  const info = page.locator(".vertex-selection-info");
  await expect(info).toContainText("точно (OCCT)");
  await expect(info).toContainText("20 · -10 · 10");
  await page.keyboard.press("Digit1");
  await page
    .getByRole("button", {
      name: "Измерить расстояние между двумя точками поверхности",
    })
    .click();
  const other = await vertex("box-vertex:xmin:ymin:zmax");
  await page.mouse.click(other.x, other.y);
  await page.mouse.click(corner.x, corner.y);
  const result = page.locator(".measure-result");
  await expect(result).toContainText("40,00");
  await expect(result).toContainText("точно (OCCT)");
  expect(
    (await calls(page, "snap_model_reference")).map((call) => call.query.kind),
  ).toEqual(["vertex", "vertex", "vertex"]);
});

test("viewport overlays never collapse or overlap in narrow areas", async ({
  page,
}, info) => {
  await mount(page, "en");
  for (const [width, height] of [
    [1440, 960],
    [1100, 800],
    [860, 700],
    [700, 620],
    [560, 520],
  ] as const) {
    await page.setViewportSize({ width, height });
    await page.waitForTimeout(250);
    const boxes = await page.evaluate(() => {
      const box = (selector: string) => {
        const element = document.querySelector(selector) as HTMLElement | null;
        if (!element || getComputedStyle(element).display === "none")
          return null;
        const rect = element.getBoundingClientRect();
        return rect.width && rect.height
          ? { x: rect.x, y: rect.y, w: rect.width, h: rect.height }
          : null;
      };
      return {
        viewport: box(".viewer-area .viewport"),
        segmented: box(".view-bottom .segmented"),
        render: box(".view-bottom .render-select"),
        toolbar: box(".view-toolbar"),
        heading: box(".viewport-heading"),
        actions: box(".topbar-actions"),
      };
    });
    await page.screenshot({ path: info.outputPath(`overlays-${width}.png`) });
    // A visible control keeps its natural size instead of collapsing to an icon.
    if (boxes.render)
      expect(boxes.render.w, `render select at ${width}`).toBeGreaterThan(110);
    if (boxes.segmented)
      expect(boxes.segmented.w, `views at ${width}`).toBeGreaterThan(150);
    const overlap = (a: typeof boxes.render, b: typeof boxes.render) =>
      !!a &&
      !!b &&
      a.x < b.x + b.w &&
      b.x < a.x + a.w &&
      a.y < b.y + b.h &&
      b.y < a.y + a.h;
    expect(
      overlap(boxes.segmented, boxes.render),
      `bottom controls at ${width}`,
    ).toBe(false);
    expect(
      overlap(boxes.toolbar, boxes.segmented),
      `toolbar and views at ${width}`,
    ).toBe(false);
    // The main menu drop-down opens unclipped wherever the menu is shown.
    const fileMenu = page.locator(".main-menu-group > button").first();
    if (await fileMenu.isVisible()) {
      await fileMenu.click();
      const dropdown = page.locator('.main-menu [role="menu"]');
      await expect(dropdown).toBeVisible();
      expect(
        (await dropdown.boundingBox())!.height,
        `menu at ${width}`,
      ).toBeGreaterThan(60);
      await page.keyboard.press("Escape");
    }
    // Top bar actions (export, settings) stay fully on screen.
    expect(
      boxes.actions!.x + boxes.actions!.w,
      `top bar at ${width}`,
    ).toBeLessThanOrEqual(width);
    // Everything stays inside the 3D area.
    for (const item of [boxes.segmented, boxes.render, boxes.toolbar])
      if (item && boxes.viewport)
        expect(item.x + item.w).toBeLessThanOrEqual(
          boxes.viewport.x + boxes.viewport.w + 1,
        );
  }
});

test("circle brush and lasso select bodies; Esc leaves the brush", async ({
  page,
}) => {
  await mount(page, "en");
  const centre = await page.evaluate(() => (window as any).__m2.bodyScreen());
  const selection = () => page.evaluate(() => (window as any).__m2.selection());
  await page.mouse.move(centre.x, centre.y);
  await page.locator('[data-editor="viewport3d"]').focus();
  // Circle brush: C, paint over the body, Esc finishes.
  await page.keyboard.press("KeyC");
  await expect(page.locator(".region-select-hint")).toBeVisible();
  await page.mouse.move(centre.x - 120, centre.y - 120);
  await page.mouse.down();
  await page.mouse.move(centre.x, centre.y, { steps: 6 });
  await page.mouse.up();
  await expect.poll(selection).toEqual(["body"]);
  await page.keyboard.press("Escape");
  await expect(page.locator(".region-select-hint")).toHaveCount(0);
  // Alt+A clears, then a Ctrl+RMB lasso around the body selects it again.
  await page.keyboard.press("Alt+KeyA");
  await expect.poll(selection).toEqual([]);
  await page.keyboard.down("Control");
  await page.mouse.move(centre.x - 150, centre.y - 120);
  await page.mouse.down({ button: "right" });
  for (const [dx, dy] of [
    [150, -120],
    [150, 120],
    [-150, 120],
    [-150, -110],
  ] as const)
    await page.mouse.move(centre.x + dx, centre.y + dy, { steps: 5 });
  await page.mouse.up({ button: "right" });
  await page.keyboard.up("Control");
  await expect.poll(selection).toEqual(["body"]);
  expect(await calls(page, "apply_ir_commands")).toHaveLength(0);
});
