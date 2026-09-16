import { test, expect, type Page } from "@playwright/test";
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    if (!localStorage.getItem("forma.locale"))
      localStorage.setItem("forma.locale", "en");
  });
});
test("typed feature editor preserves references when changing a dimension", async ({
  page,
}) => {
  await legacyProject(page, "Typed plate");
  await page.evaluate(() => {
    const projects = JSON.parse(localStorage.getItem("forma.projects.v1")!);
    projects[0].revisions[0].program = JSON.stringify(
      {
        version: 1,
        features: [
          {
            id: "SketchPlate",
            operation: { type: "rectangle", width: 60, depth: 40 },
          },
          {
            id: "Pad",
            operation: { type: "extrude", sketch: "SketchPlate", distance: 10 },
          },
        ],
        output: "Pad",
      },
      null,
      2,
    );
    localStorage.setItem("forma.projects.v1", JSON.stringify(projects));
  });
  await page.reload();
  await page
    .getByRole("button", { name: "model.cad.json", exact: true })
    .click();
  await page.getByText("SketchPlate · rectangle", { exact: true }).click();
  await page.getByLabel("SketchPlate.width").fill("80");
  await page.getByLabel("CAD source").click();
  const source = JSON.parse(await page.getByLabel("CAD source").inputValue());
  expect(source.features[0].operation.width).toBe(80);
  expect(source.features[1].operation.sketch).toBe("SketchPlate");
  await page.screenshot({
    path: "../../docs/verification/cad-feature-editor.png",
  });
});
async function legacyProject(page: Page, name = "Precision bracket") {
  await page.goto("/");
  const id = await page.evaluate((name) => {
    const id = crypto.randomUUID(),
      rid = crypto.randomUUID(),
      now = new Date().toISOString();
    localStorage.setItem(
      "forma.projects.v1",
      JSON.stringify([
        {
          schemaVersion: 1,
          id,
          name,
          units: "mm",
          agent: "codex",
          pinned: false,
          createdAt: now,
          updatedAt: now,
          currentRevision: rid,
          messages: [],
          files: [],
          exports: [],
          revisions: [
            {
              id: rid,
              parent: null,
              createdAt: now,
              prompt: "Legacy model",
              parameters: {
                kind: "bracket",
                width: 120,
                depth: 65,
                height: 60,
                thickness: 5,
                holeDiameter: 8,
                holes: 4,
              },
            },
          ],
        },
      ]),
    );
    return id;
  }, name);
  await page.goto("/#/project/" + id);
  await page.reload();
  await expect(page.locator("canvas")).toBeVisible();
}

test("create, edit with approval, restore, export, persist", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await legacyProject(page);
  await expect(
    page.getByRole("heading", { name: "What will we make?" }),
  ).toBeVisible();
  await expect(page.locator("canvas")).toBeVisible();
  await page
    .getByRole("button", { name: "Edit model parameters", exact: true })
    .click();
  await page
    .getByRole("spinbutton", { name: "width", exact: true })
    .fill("140");
  await page.getByRole("button", { name: "Review changes" }).click();
  await expect(
    page.getByRole("heading", { name: "Update model parameters" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Allow once" }).click();
  await expect(page.locator(".revision-badge")).toBeVisible();
  await page.getByRole("button", { name: "History", exact: false }).click();
  await page
    .getByRole("button", { name: "Restore", exact: true })
    .last()
    .click();
  await page.getByRole("button", { name: "Deny", exact: true }).click();
  await expect(page.locator(".revision-badge")).toBeVisible();
  await page
    .getByRole("button", { name: "Restore", exact: true })
    .last()
    .click();
  await page.getByRole("button", { name: "Allow once" }).click();
  await expect(page.locator(".revision-badge")).toHaveText("Revision 3");
  await page.getByRole("button", { name: "Export", exact: true }).click();
  await page.getByRole("button", { name: "Export STL" }).click();
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Allow once" }).click();
  expect((await download).suggestedFilename()).toBe("Precision-bracket-r3.stl");
  await page.reload();
  await expect(page.locator(".revision-badge")).toHaveText("Revision 3");
  await expect(page.locator("canvas")).toBeVisible();
  await page.screenshot({ path: "test-results/workspace.png" });
  expect(errors).toEqual([]);
  await page.getByRole("button", { name: "forma", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "New project", exact: true }),
  ).toBeVisible();
});
test("browser AI fails clearly without claiming to generate", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "New project", exact: true }).click();
  await page
    .getByRole("button", { name: "Create project", exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "Message your CAD assistant" })
    .fill("Make it 20 mm wider");
  await page.getByRole("button", { name: "Send prompt", exact: true }).click();
  await page.getByRole("button", { name: "Allow once" }).click();
  await expect(page.locator(".message.error")).toContainText("desktop app");
  await expect(page.locator(".revision-badge")).toHaveText("No revisions");
});

test("custom controls, folders and persisted confirmation policy", async ({
  page,
}) => {
  await legacyProject(page);
  await page.getByRole("button", { name: "workspace", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "model.parameters.json" }),
  ).toBeHidden();
  await page.getByRole("button", { name: "workspace", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "model.parameters.json" }),
  ).toBeVisible();
  await page.getByRole("combobox", { name: "Rendering mode" }).click();
  await page.getByRole("option", { name: "Wireframe", exact: true }).click();
  await expect(
    page.getByRole("combobox", { name: "Rendering mode" }),
  ).toContainText("Wireframe");
  await page
    .getByRole("button", { name: "Settings", exact: true })
    .first()
    .click();
  await page.getByRole("button", { name: "Permissions", exact: true }).click();
  await page.getByRole("combobox", { name: "Confirmation mode" }).click();
  await page
    .getByRole("option", { name: "No confirmations", exact: true })
    .click();
  await expect(
    page.getByRole("checkbox", { name: "Edit models and project files" }),
  ).not.toBeChecked();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await page
    .getByRole("button", { name: "Edit model parameters", exact: true })
    .click();
  const width = page.getByRole("spinbutton", { name: "width", exact: true });
  await width.fill("140");
  await page
    .getByRole("button", { name: "Increase width", exact: true })
    .click();
  await expect(width).toHaveValue("140.1");
  await page.getByRole("button", { name: "Review changes" }).click();
  await expect(page.locator(".revision-badge")).toHaveText("Revision 2");
  await expect(page.getByRole("button", { name: "Allow once" })).toBeHidden();
  await page.reload();
  await page
    .getByRole("button", { name: "Settings", exact: true })
    .first()
    .click();
  await page.getByRole("button", { name: "Permissions", exact: true }).click();
  await expect(
    page.getByRole("combobox", { name: "Confirmation mode" }),
  ).toContainText("No confirmations");
  await page
    .getByRole("checkbox", { name: "Edit models and project files" })
    .click();
  await expect(
    page.getByRole("checkbox", { name: "Edit models and project files" }),
  ).toBeChecked();
  await page.screenshot({ path: "test-results/custom-confirmations.png" });
});

test("agent and CAD settings show distinct tools", async ({ page }) => {
  await page.goto("/");
  await page
    .getByRole("button", { name: "Settings", exact: true })
    .first()
    .click();
  await expect(page.locator(".health-row")).toContainText(["Desktop runtime"]);
  await expect(
    page.locator(".health-row").filter({ hasText: "CAD kernel" }),
  ).toHaveCount(0);
  await page
    .getByRole("button", { name: "CAD environment", exact: true })
    .click();
  await expect(
    page.locator(".health-row").filter({ hasText: "CAD kernel" }),
  ).toBeVisible();
  await expect(
    page.locator(".health-row").filter({ hasText: "Desktop runtime" }),
  ).toHaveCount(0);
});

test("viewport clicks and tools preserve camera zoom", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "New project", exact: true }).click();
  await page
    .getByRole("button", { name: "Create project", exact: true })
    .click();
  const canvas = page.locator("canvas");
  await canvas.waitFor();
  await page.waitForTimeout(1200);
  await canvas.hover();
  await page.mouse.wheel(0, -180);
  let prior: number[] | undefined;
  let stable = 0;
  await expect
    .poll(
      async () => {
        const state = JSON.parse(
          (await canvas.getAttribute("data-camera")) ?? "{}",
        );
        if (!state.position) return 0;
        const delta = prior
          ? Math.hypot(
              ...state.position.map((v: number, i: number) => v - prior![i]),
            )
          : Infinity;
        prior = state.position;
        stable = delta < 0.001 ? stable + 1 : 0;
        return stable;
      },
      { intervals: [500], timeout: 20000 },
    )
    .toBeGreaterThanOrEqual(3);
  const before = JSON.parse((await canvas.getAttribute("data-camera")) ?? "{}");
  await page
    .getByRole("button", { name: "Select object", exact: true })
    .click();
  await canvas.click({ position: { x: 40, y: 240 } });
  await page.waitForTimeout(1000);
  const after = JSON.parse((await canvas.getAttribute("data-camera")) ?? "{}");
  expect(
    Math.hypot(
      ...after.position.map((v: number, i: number) => v - before.position[i]),
    ),
  ).toBeLessThan(0.1);
  expect(after.zoom).toBe(before.zoom);
});

test("new projects are empty without a template selector", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "New project", exact: true }).click();
  await expect(page.getByText("Starting point", { exact: true })).toHaveCount(
    0,
  );
  await page
    .getByRole("button", { name: "Create project", exact: true })
    .click();
  await expect(page.locator(".revision-badge")).toHaveText("No revisions");
  await expect(
    page.getByRole("button", { name: "model.py", exact: true }),
  ).toHaveCount(0);
  await page.reload();
  await expect(page.locator(".revision-badge")).toHaveText("No revisions");
});

test("quota has one actionable error card", async ({ page }) => {
  await legacyProject(page);
  await page.evaluate(() => {
    const all = JSON.parse(localStorage.getItem("forma.projects.v1")!);
    all[0].messages = [
      {
        id: crypto.randomUUID(),
        role: "error",
        text: "You've hit your usage limit. Upgrade to Pro (https://chatgpt.com/explore/pro) or try again at 8:55 PM.",
        createdAt: new Date().toISOString(),
      },
    ];
    localStorage.setItem("forma.projects.v1", JSON.stringify(all));
  });
  await page.reload();
  await expect(page.locator(".quota-message")).toHaveCount(1);
  await expect(page.locator(".quota-message")).toContainText("8:55 PM");
  await expect(
    page.getByRole("link", { name: "Limits and credits" }),
  ).toHaveAttribute("href", "https://chatgpt.com/codex/settings/usage");
  await expect(
    page.getByText("Check CLI login and dependency setup."),
  ).toHaveCount(0);
  await expect(page.locator("canvas")).toBeVisible();
  await page.waitForTimeout(1200);
  await page.screenshot({ path: "test-results/quota-message.png" });
});

test("camera presets apply their intended direction", async ({ page }) => {
  await legacyProject(page);
  const canvas = page.locator("canvas");
  await page.getByRole("button", { name: "Top", exact: true }).click();
  await expect
    .poll(
      async () => {
        const camera = JSON.parse(
          (await canvas.getAttribute("data-camera")) ?? "{}",
        );
        return camera.position
          ? Math.abs(camera.position[0]) + Math.abs(camera.position[2])
          : 1000;
      },
      { timeout: 15000 },
    )
    .toBeLessThan(0.1);
  await page.getByRole("button", { name: "Reset camera", exact: true }).click();
  await expect
    .poll(async () => {
      const camera = JSON.parse(
        (await canvas.getAttribute("data-camera")) ?? "{}",
      );
      return camera.position?.[0] ?? 0;
    })
    .toBeGreaterThan(10);
});

test("viewport toolbars never overlap and body visibility toggles", async ({
  page,
}) => {
  await legacyProject(page);
  await page.evaluate(() => {
    const all = JSON.parse(localStorage.getItem("forma.projects.v1")!);
    const text =
      "Добавь анимацию движения к текущей сборке. Сохрани геометрию, раздели детали через parts.";
    all[0].messages.push({
      id: crypto.randomUUID(),
      role: "user",
      text,
      createdAt: new Date().toISOString(),
    });
    all[0].revisions[0].prompt = text;
    localStorage.setItem("forma.projects.v1", JSON.stringify(all));
  });
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Play motion", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByText("Добавить движение", { exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByText("Добавь анимацию движения к текущей сборке.", {
      exact: false,
    }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "History", exact: false }).click();
  await expect(page.getByText("Assembly motion configured")).toBeVisible();
  await page.getByRole("button", { name: "Files", exact: false }).click();
  await expect(
    page.getByRole("button", { name: /^Hide / }).first(),
  ).toBeVisible();
  const hide = page.getByRole("button", { name: /^Hide / }).first();
  const name = (await hide.getAttribute("aria-label"))!.replace("Hide ", "");
  await hide.click();
  await expect(
    page.getByRole("button", { name: "Show " + name, exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Show " + name, exact: true }).click();
  await page
    .getByRole("button", {
      name: "Measure distance between two surface points",
      exact: true,
    })
    .click();
  for (const width of [1440, 960]) {
    await page.setViewportSize({ width, height: 940 });
    const main = await page.locator(".view-toolbar").boundingBox();
    const measure = await page.locator(".measure-result").boundingBox();
    expect(measure!.y).toBeGreaterThanOrEqual(main!.y + main!.height + 9);
  }
});
