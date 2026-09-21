import { expect, test } from "@playwright/test";

test("Russian is default; language and theme persist without changing project content", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("lang", "ru");
  await expect(
    page.getByRole("heading", { name: "Место для ваших идей." }),
  ).toBeVisible();
  await expect(page.locator(".button.primary").first()).toHaveCSS(
    "background-color",
    "rgb(124, 58, 237)",
  );
  await page.screenshot({
    path: "../../docs/verification/1.1.0-dashboard-ru.png",
  });
  await page
    .getByRole("button", { name: "Настройки", exact: true })
    .first()
    .click();
  await page.getByRole("button", { name: "Внешний вид", exact: true }).click();
  await expect(page.getByRole("combobox", { name: "Язык" })).toBeVisible();
  await page.screenshot({
    path: "../../docs/verification/1.1.0-settings-ru.png",
  });
  await page.getByRole("combobox", { name: "Язык" }).click();
  await page.getByRole("option", { name: "English", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Appearance", exact: true }),
  ).toBeVisible();
  await page.getByRole("combobox").last().click();
  await page.getByRole("option", { name: "Light", exact: true }).click();
  await expect(page.locator(".select-menu")).toHaveCount(0);
  await page.screenshot({
    path: "../../docs/verification/1.1.0-settings-en-light.png",
  });
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(
    page.getByRole("heading", { name: "A place to make things." }),
  ).toBeVisible();
});

test("Russian workspace translates features, tooltips and confirmations but preserves model IDs", async ({
  page,
}) => {
  await page.goto("/");
  const id = await page.evaluate(() => {
    const id = crypto.randomUUID(),
      rid = crypto.randomUUID(),
      now = new Date().toISOString();
    localStorage.setItem(
      "forma.projects.v1",
      JSON.stringify([
        {
          id,
          name: "My part.step",
          schemaVersion: 1,
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
              prompt: "User's model",
              parameters: {
                kind: "box",
                width: 60,
                depth: 40,
                height: 10,
                thickness: 2,
                holeDiameter: 0,
                holes: 0,
              },
              program: JSON.stringify({
                version: 1,
                features: [
                  {
                    id: "Sketch001",
                    operation: { type: "rectangle", width: 60, depth: 40 },
                  },
                  {
                    id: "Pad001",
                    operation: {
                      type: "extrude",
                      sketch: "Sketch001",
                      distance: 10,
                    },
                  },
                ],
                output: "Pad001",
              }),
            },
          ],
        },
      ]),
    );
    return id;
  });
  await page.goto(`/#/project/${id}`);
  await page.reload();
  await expect(page.locator("canvas")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Вписать модель", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: "../../docs/verification/1.1.0-workspace-ru.png",
  });
  await page
    .getByRole("button", { name: "model.cad.json", exact: true })
    .click();
  await page.getByText("Sketch001 · Прямоугольник", { exact: true }).click();
  await page.getByLabel("Sketch001.Ширина").fill("80");
  await page.getByLabel("Исходник CAD-модели").click();
  const document = JSON.parse(
    await page.getByLabel("Исходник CAD-модели").inputValue(),
  );
  expect(document.features[0].operation.width).toBe(80);
  expect(document.features[1].operation.sketch).toBe("Sketch001");
  await page.screenshot({
    path: "../../docs/verification/1.1.0-features-ru.png",
  });
  await page
    .getByRole("button", { name: "Закрыть диалог", exact: true })
    .click();
  await page.getByRole("button", { name: "Настройки", exact: true }).click();
  await page.getByRole("button", { name: "Разрешения", exact: true }).click();
  await expect(
    page.getByText("Изменение моделей и файлов проекта", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Connect to CLI agent", { exact: true }),
  ).toHaveCount(0);
});
