import { expect, test, type Page } from "@playwright/test";

async function theme(page: Page, name: "Светлая" | "Тёмная") {
  await page.getByRole("button", { name: "Настройки", exact: true }).first().click();
  await page.getByRole("button", { name: "Внешний вид", exact: true }).click();
  await page.getByRole("combobox").last().click();
  await page.getByRole("option", { name, exact: true }).click();
  await page.getByRole("button", { name: "Закрыть диалог", exact: true }).click();
  await expect(page.locator(".modal")).toHaveCount(0);
}

async function corner(page: Page) {
  // Read the composited frame: WebGL canvases may not retain a drawing buffer
  // for drawImage(canvas) on Linux's software renderer.
  const frame = (await page.locator("canvas").screenshot()).toString("base64");
  return page.evaluate(async (png) => {
    const image = new Image();
    image.src = `data:image/png;base64,${png}`;
    await image.decode();
    const copy = document.createElement("canvas");
    copy.width = image.width; copy.height = image.height;
    const ctx = copy.getContext("2d")!;
    ctx.drawImage(image, 0, 0);
    return [...ctx.getImageData(5, 5, 1, 1).data].slice(0, 3);
  }, frame);
}

test("light surfaces and WebGL switch together; dark appearance and camera survive", async ({ page }) => {
  test.setTimeout(90000);
  await page.goto("/");
  const id = await page.evaluate(() => {
    const id = crypto.randomUUID(), rid = crypto.randomUUID(), now = new Date().toISOString();
    localStorage.setItem("forma.projects.v1", JSON.stringify([{
      id, name: "Theme check", schemaVersion: 1, units: "mm", agent: "codex",
      pinned: false, createdAt: now, updatedAt: now, currentRevision: rid,
      messages: [], files: [], exports: [], revisions: [{
        id: rid, parent: null, createdAt: now, prompt: "Model",
        parameters: {kind: "box", width: 60, depth: 40, height: 10, thickness: 2, holeDiameter: 0, holes: 0},
      }],
    }]));
    return id;
  });
  await page.goto(`/#/project/${id}`);
  await page.reload();
  await expect(page.locator("canvas")).toBeVisible();
  await expect.poll(async () => Math.max(...(await corner(page)).map((v, i) => Math.abs(v - [29, 32, 36][i])))).toBeLessThan(4);
  const selectors = [".view-toolbar", ".segmented", ".render-select", ".composer", ".welcome-icon", ".document-tabs", ".model-bottom"];
  // Record existing dark styles so every selected surface must round-trip exactly.
  const surfaces = async () => page.evaluate((selectors) => selectors.flatMap(s => {
    const node = document.querySelector(s);
    return node ? [{selector:s, background:getComputedStyle(node).backgroundColor, color:getComputedStyle(node).color}] : [];
  }), selectors);
  const dark = await surfaces();
  // Wait for the initial animated fit to settle before comparing theme changes.
  let camera = await page.locator("canvas").getAttribute("data-camera");
  let stableSamples = 0;
  await expect.poll(async () => {
    const next = await page.locator("canvas").getAttribute("data-camera");
    stableSamples = next === camera ? stableSamples + 1 : 0;
    camera = next;
    return stableSamples >= 3;
  }, { intervals: [500, 500, 500] }).toBe(true);
  await theme(page, "Светлая");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect.poll(async () => Math.min(...await corner(page))).toBeGreaterThan(230);
  for (const surface of await surfaces()) {
    const rgb = surface.background.match(/\d+/g)!.map(Number);
    if (rgb.length === 4 && rgb[3] === 0) continue;
    expect(Math.min(...rgb.slice(0, 3)), surface.selector).toBeGreaterThan(210);
  }
  await page.screenshot({path: "../../docs/verification/1.0.0-workspace-light.png"});
  await theme(page, "Тёмная");
  await expect.poll(async () => Math.max(...(await corner(page)).map((v, i) => Math.abs(v - [29, 32, 36][i])))).toBeLessThan(4);
  await expect.poll(surfaces).toEqual(dark);
  expect(await page.locator("canvas").getAttribute("data-camera")).toEqual(camera);
  await page.screenshot({path: "../../docs/verification/1.0.0-workspace-dark.png"});
  await theme(page, "Светлая");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect.poll(async () => Math.min(...await corner(page))).toBeGreaterThan(230);
});
