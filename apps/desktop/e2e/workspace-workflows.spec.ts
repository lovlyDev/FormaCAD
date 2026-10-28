import { expect, test } from "@playwright/test";
import { selectValue } from "./customSelect";
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    if (localStorage.getItem("m1-workflow-seeded")) return;
    const now = new Date().toISOString();
    const program = JSON.stringify({
      schemaVersion: 2,
      revisionId: "r1",
      parameters: [{ id: "width_parameter", name: "Named width", valueMm: 60 }],
      features: [
        {
          id: "profile",
          name: "Base profile",
          operation: {
            type: "rectangle",
            width: { kind: "parameter", parameterId: "width_parameter" },
            depth: { kind: "literal", mm: 40 },
          },
        },
        {
          id: "pad",
          name: "Named pad",
          operation: {
            type: "extrude",
            sketchId: "profile",
            distance: { kind: "literal", mm: 20 },
          },
        },
      ],
      bodies: [{ id: "body", name: "Fixture body", sourceFeatureId: "pad" }],
    });
    const project = (id: string) => ({
      schemaVersion: 1,
      id,
      name: id,
      units: "mm",
      agent: "codex",
      pinned: false,
      createdAt: now,
      updatedAt: now,
      currentRevision: "r1",
      messages: [],
      files: [],
      exports: [],
      revisions: [
        {
          id: "r1",
          parent: null,
          createdAt: now,
          prompt: "Baseline revision",
          program,
          parameters: {
            kind: "box",
            width: 60,
            depth: 40,
            height: 20,
            thickness: 2,
            holeDiameter: 0,
            holes: 0,
          },
        },
      ],
    });
    localStorage.setItem(
      "forma.projects.v1",
      JSON.stringify([project("first-workflow"), project("second-workflow")]),
    );
    localStorage.setItem("forma.locale", "en");
    localStorage.setItem("forma.theme", "dark");
    localStorage.setItem("m1-workflow-seeded", "1");
  });
  await page.goto("/#/project/first-workflow");
  await expect(page.locator('[data-editor="viewport3d"] canvas')).toBeVisible();
});
test("custom workspaces and editor navigation preserve the project document", async ({
  page,
}) => {
  const original = await page.evaluate(() =>
    localStorage.getItem("forma.projects.v1"),
  );
  await page
    .getByRole("button", { name: "Create workspace", exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "Workspace name" })
    .fill("Производство α");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(
    page.getByRole("tab", { name: "Производство α", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page
    .locator(".outliner-unassigned")
    .getByRole("button", { name: /Named pad/ })
    .click();
  await expect(page.locator('[data-editor="ai"]')).toBeVisible();
  await page.keyboard.press("F3");
  const input = page.getByRole("textbox", {
    name: "Search commands and projects…",
  });
  await input.fill("Named pad");
  await input.press("Enter");
  await expect(page.locator('[data-editor="properties"]')).toBeVisible();
  await expect(
    page
      .locator('[data-editor="properties"]')
      .getByRole("heading", { name: "Named pad" }),
  ).toBeVisible();
  await page.keyboard.press("F3");
  await input.fill("Named width");
  await input.press("Enter");
  await expect(page.locator('[data-editor="parameters"]')).toContainText(
    "Named width",
  );
  await page.reload();
  await expect(
    page.getByRole("tab", { name: "Производство α", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await selectValue(
    page,
    page.getByRole("combobox", { name: "Switch project", exact: true }),
    "second-workflow",
  );
  await expect(page.locator('[data-editor="ai"]')).toBeVisible();
  await expect(
    page.getByRole("tab", { name: "Производство α", exact: true }),
  ).toHaveCount(0);
  await page.getByRole("tab", { name: "first-workflow", exact: true }).click();
  await expect(
    page.getByRole("tab", { name: "Производство α", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("F3");
  await input.fill("Settings: Navigation");
  await input.press("Enter");
  await expect(
    page
      .getByRole("dialog")
      .getByRole("heading", { name: "Navigation", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Close dialog", exact: true }).click();
  expect(
    await page.evaluate(() => localStorage.getItem("forma.projects.v1")),
  ).toBe(original);
});
test("input, navigation and interface settings persist without remounting the viewport", async ({
  page,
}) => {
  await page.evaluate(() => {
    (
      window as typeof window & { settingsCanvas?: Element | null }
    ).settingsCanvas = document.querySelector(
      '[data-editor="viewport3d"] canvas',
    );
  });
  await page
    .getByRole("button", { name: "Settings", exact: true })
    .first()
    .click();
  const dialog = page.getByRole("dialog");
  await dialog.getByRole("button", { name: "Interface", exact: true }).click();
  await selectValue(
    page,
    dialog.getByRole("combobox", { name: "Interface density" }),
    "compact",
  );
  await expect(page.locator("html")).toHaveAttribute("data-density", "compact");
  await dialog.getByRole("button", { name: "Input", exact: true }).click();
  await dialog
    .getByRole("checkbox", { name: "Emulate numpad with number keys" })
    .click();
  await dialog.getByRole("button", { name: "Navigation", exact: true }).click();
  await dialog
    .getByRole("checkbox", { name: "Orbit around selection" })
    .click();
  await dialog.getByRole("checkbox", { name: "Zoom toward pointer" }).click();
  await page.getByRole("button", { name: "Close dialog", exact: true }).click();
  expect(
    await page.evaluate(
      () =>
        (window as typeof window & { settingsCanvas?: Element | null })
          .settingsCanvas ===
        document.querySelector('[data-editor="viewport3d"] canvas'),
    ),
  ).toBe(true);
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-density", "compact");
  await page
    .getByRole("button", { name: "Settings", exact: true })
    .first()
    .click();
  await dialog.getByRole("button", { name: "Navigation", exact: true }).click();
  await expect(
    dialog.getByRole("checkbox", { name: "Orbit around selection" }),
  ).toBeChecked();
  await expect(
    dialog.getByRole("checkbox", { name: "Zoom toward pointer" }),
  ).toBeChecked();
});
