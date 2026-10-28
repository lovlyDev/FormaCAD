import { expect, test } from "@playwright/test";
for (const locale of ["ru", "en"] as const)
  for (const theme of ["dark", "light"] as const) {
    test(`workspace areas preserve the live viewport: ${locale} ${theme}`, async ({
      page,
    }, info) => {
      const id = "m1-workspace-test";
      await page.addInitScript(
        ({ locale, theme, id }) => {
          if (localStorage.getItem("m1-seeded")) return;
          const now = new Date().toISOString();
          const project = (id: string, name: string) => ({
            schemaVersion: 1,
            id,
            name,
            units: "mm",
            agent: "codex",
            pinned: false,
            createdAt: now,
            updatedAt: now,
            currentRevision: "revision-1",
            messages: [],
            files: [],
            exports: [],
            revisions: [
              {
                id: "revision-1",
                parent: null,
                createdAt: now,
                prompt: "test geometry",
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
            JSON.stringify([
              project(id, "Area fixture"),
              project("m1-second-project", "Second fixture"),
            ]),
          );
          localStorage.setItem("forma.locale", locale);
          localStorage.setItem("forma.theme", theme);
          localStorage.setItem("m1-seeded", "1");
        },
        { locale, theme, id },
      );
      const label = (ru: string, en: string) => (locale === "ru" ? ru : en);
      const errors: string[] = [];
      page.on("pageerror", (error) => errors.push(error.message));
      await page.goto(`/#/project/${id}`);
      const viewport = page.locator('[data-editor="viewport3d"]');
      const canvas = viewport.locator("canvas");
      await expect(canvas).toBeVisible();
      await expect
        .poll(() =>
          page.evaluate(
            () =>
              !!localStorage.getItem(
                "forma.ui.project.m1-workspace-test.view.main.camera",
              ),
          ),
        )
        .toBe(true);
      await page.evaluate(() => {
        const state = window as typeof window & {
          m1Canvas?: HTMLCanvasElement;
          m1Context?: WebGL2RenderingContext | null;
        };
        state.m1Canvas = document.querySelector(
          '[data-editor="viewport3d"] canvas',
        )!;
        state.m1Context = state.m1Canvas.getContext("webgl2");
      });
      const originalCamera = await page.evaluate(() =>
        JSON.parse(
          localStorage.getItem(
            "forma.ui.project.m1-workspace-test.view.main.camera",
          )!,
        ),
      );
      const layoutStart = Date.now();
      await viewport
        .getByRole("button", {
          name: label(
            "Разделить область по вертикали",
            "Split area vertically",
          ),
          exact: true,
        })
        .click();
      await expect(page.locator(".workspace-editor:not([hidden])")).toHaveCount(
        5,
      );
      await page.evaluate(
        () =>
          new Promise<void>((resolve) =>
            requestAnimationFrame(() => resolve()),
          ),
      );
      await info.attach("layout-timing", {
        body: JSON.stringify({
          locale,
          theme,
          splitToNextFrameMs: Date.now() - layoutStart,
        }),
        contentType: "application/json",
      });
      await viewport
        .getByRole("button", {
          name: label("Развернуть область", "Maximize area"),
          exact: true,
        })
        .click();
      await expect(page.locator(".workspace-editor:not([hidden])")).toHaveCount(
        1,
      );
      await viewport
        .getByRole("button", {
          name: label("Восстановить область", "Restore area"),
          exact: true,
        })
        .click();
      const extra = page.locator('[data-editor="parameters"]');
      await extra
        .getByRole("button", {
          name: label(
            "Объединить область с соседней",
            "Join area with neighbor",
          ),
          exact: true,
        })
        .click();
      await expect(page.locator(".workspace-editor:not([hidden])")).toHaveCount(
        4,
      );
      const outliner = page.locator('[data-editor="outliner"]');
      await outliner
        .getByRole("combobox", { name: label("Тип редактора", "Editor type") })
        .click();
      await page
        .getByRole("option", {
          name: label("Свойства", "Properties"),
          exact: true,
        })
        .click();
      await expect(page.locator('[data-editor="properties"]')).toBeVisible();
      await expect(page.locator('[data-editor="ai"]')).toBeVisible();
      const separator = page.getByRole("separator").first();
      await separator.focus();
      await page.keyboard.press("ArrowRight");
      await viewport.focus();
      await page.keyboard.press("KeyN");
      await expect(
        viewport.getByRole("complementary", {
          name: label("Боковая панель 3D-вида", "Viewport sidebar"),
        }),
      ).toBeVisible();
      await page.keyboard.press("KeyN");
      await page.keyboard.press("KeyT");
      await expect(viewport.locator(".viewport-toolbars")).toBeHidden();
      await page.keyboard.press("KeyT");
      await page.evaluate(async () => {
        const { applyTheme } = await import("/src/lib/theme.ts");
        applyTheme(
          document.documentElement.dataset.theme === "dark" ? "light" : "dark",
        );
      });
      await expect
        .poll(() =>
          page.evaluate(() => {
            const state = window as typeof window & {
              m1Canvas?: HTMLCanvasElement;
              m1Context?: WebGL2RenderingContext | null;
            };
            const canvas = document.querySelector(
              '[data-editor="viewport3d"] canvas',
            ) as HTMLCanvasElement;
            return (
              !!state.m1Context &&
              canvas === state.m1Canvas &&
              canvas.getContext("webgl2") === state.m1Context &&
              !state.m1Context?.isContextLost()
            );
          }),
        )
        .toBe(true);
      const camera = await page.evaluate(() =>
        JSON.parse(
          localStorage.getItem(
            "forma.ui.project.m1-workspace-test.view.main.camera",
          )!,
        ),
      );
      expect(camera.position).toEqual(originalCamera.position);
      expect(camera.target).toEqual(originalCamera.target);
      await page.screenshot({
        path: info.outputPath(`areas-${locale}-${theme}.png`),
      });
      const persisted = await page.evaluate(() =>
        localStorage.getItem(
          "forma.ui.project.m1-workspace-test.areas.v1.workspaces",
        ),
      );
      expect(persisted).toBeTruthy();
      await page.reload();
      await expect(page.locator('[data-editor="properties"]')).toBeVisible();
      await expect(
        page.locator('[data-editor="viewport3d"] canvas'),
      ).toBeVisible();
      await page.keyboard.press("F3");
      await expect(
        page.getByRole("heading", {
          name: label("Палитра команд", "Command palette"),
        }),
      ).toBeVisible();
      await page.keyboard.press("Escape");
      expect(errors).toEqual([]);
    });
  }
