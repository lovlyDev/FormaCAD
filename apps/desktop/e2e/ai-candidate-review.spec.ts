import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { nativePreviewFixture } from "./nativePreviewFixture";
const fixture = nativePreviewFixture();
const targetMetrics = fixture ? JSON.parse(fs.readFileSync(path.join(fixture.stage, "12/result.json"), "utf8")) : null;

for (const locale of ["ru", "en"] as const) test(`AI candidate review ${locale}: real geometry, comparison, themes and stale rejection`, async ({ page }, info) => {
  test.skip(!fixture, "Requires the locally built Windows native CAD worker");
  const labels = locale === "ru" ? { accept: "Принять кандидат", reject: "Отклонить кандидат", current: "Текущая модель", candidate: "AI-кандидат", details: "Подробные изменения CAD", source: "Исходный CAD-код кандидата" }
    : { accept: "Accept candidate", reject: "Reject candidate", current: "Current model", candidate: "AI candidate", details: "Detailed CAD changes", source: "Candidate CAD source" };
  await page.addInitScript(locale => localStorage.setItem("forma.locale", locale), locale);
  await page.goto("/");
  let calls = 0;
  await page.route("**/__candidate_preview", async route => {
    const request = route.request().postDataJSON();
    expect(request.expectedRevision).toBe(fixture!.document.revisionId);
    const result = JSON.parse(fs.readFileSync(path.join(fixture!.stage, "12/result.json"), "utf8"));
    const header = Buffer.from(JSON.stringify({ protocolVersion: 1, sourceSha256: crypto.createHash("sha256").update(request.program).digest("hex"),
      metrics: { volumeMm3: result.volumeMm3, areaMm2: result.areaMm2, faceCount: result.faceCount, edgeCount: result.edgeCount, boundsMm: result.boundsMm } }));
    const prefix = Buffer.alloc(4); prefix.writeUInt32LE(header.length); calls++;
    await new Promise(resolve => setTimeout(resolve, 300));
    await route.fulfill({ contentType: "application/octet-stream", body: Buffer.concat([prefix, header, fs.readFileSync(path.join(fixture!.stage, "12/preview.glb"))]) });
  });
  await page.evaluate(async fixture => {
    const React = await import("/node_modules/.vite/deps/react.js"), ReactDOM = await import("/node_modules/.vite/deps/react-dom_client.js");
    const { ActionReviewDialog } = await import("/src/app/dialogs/ActionReviewDialog.tsx");
    const { useWorkspace, newProject } = await import("/src/stores/workspace.ts");
    const project = newProject("Candidate review", "blank", "mm", "codex");
    project.currentRevision = fixture.document.revisionId;
    project.revisions = [{ id: project.currentRevision, parent: null, createdAt: project.createdAt, prompt: "Baseline", parameters: { kind: "blank", width: 1, depth: 1, height: 1, thickness: 1, holeDiameter: 0, holes: 0 }, preview: "base.glb", program: JSON.stringify(fixture.document) }];
    project.files = [{ name: "base.glb", kind: "model", size: 100, data: fixture.glb }];
    useWorkspace.getState().setProject(project);
    const candidate = structuredClone(fixture.document); candidate.revisionId = "candidate";
    candidate.parameters.find((parameter: { id: string }) => parameter.id === "thickness").valueMm = 12;
    candidate.features.find((feature: { operation: { type: string } }) => feature.operation.type === "hole").name += " revised";
    candidate.bodies[0].name += " revised";
    window.isTauri = true;
    window.__TAURI_INTERNALS__ = { metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } }, invoke: async (command: string, args: unknown) => command === "preview_model"
      ? (await fetch("/__candidate_preview", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(args) })).arrayBuffer() : null };
    window.__candidateBaseline = JSON.stringify(project); window.__candidateDecisions = [];
    const root = ReactDOM.default.createRoot(document.body.appendChild(document.createElement("div")));
    const reviewPlan = { assumptions: ["Dimensions use mm"], dimensions: [{ name: "Thickness", valueMm: 12, parameterId: "thickness" }], affectedBodyIds: candidate.bodies.map((body: { id: string }) => body.id), expectedChecks: [{ type: "validSolid" }, { type: "bodyCount", count: candidate.bodies.length }, { type: "bounds", sizeMm: fixture.targetMetrics.boundsMm, toleranceMm: 0.001 }, { type: "volume", valueMm3: fixture.targetMetrics.volumeMm3, toleranceMm3: 0.001 }] };
    const pending = { title: "", description: "Candidate review", detail: JSON.stringify(candidate), run: async () => {}, review: { base: project, source: JSON.stringify(candidate), reviewPlan } };
    const render = (action: typeof pending | null) => root.render(React.default.createElement(ActionReviewDialog, { pending: action,
      onDecision: (allow: boolean) => { window.__candidateDecisions.push(allow); render(null); } }));
    window.__openCandidateReview = () => render(pending);
    window.__setWrongExpectedVolume = (wrong: boolean) => { pending.review.reviewPlan = { ...reviewPlan, expectedChecks: reviewPlan.expectedChecks.map(check => check.type === "volume" ? { ...check, valueMm3: fixture.targetMetrics.volumeMm3 + (wrong ? 1000 : 0) } : check) }; render(pending); };
    window.__openCandidateReview();
  }, { ...fixture!, targetMetrics });
  const dialog = page.locator(".candidate-review-modal"), canvas = dialog.locator("canvas");
  await expect(page.getByRole("button", { name: labels.accept, exact: true })).toBeDisabled();
  await expect(canvas).toBeVisible();
  await expect(page.getByRole("button", { name: labels.accept, exact: true })).toBeEnabled();
  await expect(dialog.locator(".candidate-plan-checks [data-check-state='passed']")).toHaveCount(4);
  await page.evaluate(() => window.__setWrongExpectedVolume(true));
  await expect(page.getByRole("button", { name: labels.accept, exact: true })).toBeDisabled();
  await expect(dialog.locator(".candidate-plan-checks [data-check-state='failed']")).toHaveCount(1);
  await page.evaluate(() => window.__setWrongExpectedVolume(false));
  await expect(page.getByRole("button", { name: labels.accept, exact: true })).toBeEnabled();
  expect(await dialog.evaluate(element => getComputedStyle(element).animationDuration)).toBe("0.2s");
  await expect(dialog.locator(".candidate-parameter-changes .candidate-change")).toContainText(locale === "ru" ? "8,00 → 12,00" : "8.00 → 12.00");
  await expect(dialog.locator(".typed-feature-icon svg")).toHaveCount(2);
  await expect.poll(async () => JSON.parse(await canvas.getAttribute("data-preview-camera") ?? "{}").moving).toBe(false);
  const box = (await canvas.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2); await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 40, box.y + box.height / 2 + 15, { steps: 8 }); await page.mouse.up();
  const camera = await canvas.getAttribute("data-preview-camera");
  for (const theme of ["dark", "light"]) {
    await page.evaluate(async theme => { const { applyTheme } = await import("/src/lib/theme.ts"); applyTheme(theme); }, theme);
    await expect.poll(() => canvas.getAttribute("data-preview-camera")).toBe(camera);
    await dialog.screenshot({ path: info.outputPath(`candidate-${locale}-${theme}.png`) });
  }
  await page.getByRole("button", { name: labels.current, exact: true }).click();
  await expect(canvas).toBeVisible();
  await page.getByRole("button", { name: labels.candidate, exact: true }).click();
  await expect(canvas).toBeVisible(); expect(calls).toBe(1);
  for (const name of [labels.details, labels.source]) {
    const summary = dialog.locator("summary").filter({ hasText: name });
    await summary.click(); await expect(summary.locator("..")).toHaveAttribute("data-expanded", "true");
    await summary.click(); await expect(summary.locator("..")).toHaveAttribute("data-expanded", "false");
    await summary.click(); await expect(summary.locator("..")).toHaveAttribute("data-expanded", "true");
    expect(await summary.locator("..").locator(":scope > .editor-disclosure-body").getAttribute("data-animation-count")).not.toBeNull();
    expect(await summary.locator("svg").last().evaluate(element => getComputedStyle(element).transitionDuration)).toBe("0.26s");
  }
  await page.emulateMedia({ reducedMotion: "reduce" });
  const sourceSummary = dialog.locator("summary").filter({ hasText: labels.source });
  await sourceSummary.click(); await sourceSummary.click();
  expect(await sourceSummary.locator("..").locator(":scope > .editor-disclosure-body").evaluate(element => element.getAnimations().length)).toBe(0);
  // The project's global reduced-motion rule uses a 0.01ms !important transition.
  expect(await sourceSummary.locator("svg").last().evaluate(element => parseFloat(getComputedStyle(element).transitionDuration))).toBeLessThanOrEqual(0.001);
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.setViewportSize({ width: 700, height: 820 });
  await dialog.locator(".candidate-review-scroll").evaluate(element => element.scrollTop = 0);
  await dialog.screenshot({ path: info.outputPath(`candidate-${locale}-narrow.png`) });
  expect(await dialog.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
  expect(await page.evaluate(async () => { const { useWorkspace } = await import("/src/stores/workspace.ts"); return JSON.stringify(useWorkspace.getState().project) === window.__candidateBaseline; })).toBe(true);
  await page.getByRole("button", { name: labels.accept, exact: true }).click();
  await expect(dialog).toHaveCount(0);
  expect(await page.evaluate(() => window.__candidateDecisions)).toEqual([true]);
  await page.evaluate(() => window.__openCandidateReview());
  await expect(page.getByRole("button", { name: labels.accept, exact: true })).toBeEnabled();
  await page.evaluate(async () => { const { useWorkspace } = await import("/src/stores/workspace.ts"); useWorkspace.getState().setProject({ ...useWorkspace.getState().project!, id: "another-project" }); });
  await expect(dialog.getByRole("alert")).toBeVisible();
  await expect(page.getByRole("button", { name: labels.accept, exact: true })).toBeDisabled();
  await page.getByRole("button", { name: labels.reject, exact: true }).click();
  expect(await page.evaluate(() => window.__candidateDecisions)).toEqual([true, false]);
});
