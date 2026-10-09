import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
function workerFixture() {
  const worker = path.resolve("src-tauri/target/debug/forma-cad-worker.exe");
  if (process.platform !== "win32" || !fs.existsSync(worker)) return null;
  const stage = fs.mkdtempSync(path.resolve("../../.local/e2e-owner-references-")), executable = path.join(stage, "forma-cad-worker.exe"); fs.copyFileSync(worker, executable);
  let sequence = 0;
  return { build(document: unknown) {
    const cwd = path.join(stage, String(sequence++)); fs.mkdirSync(cwd);
    execFileSync(executable, [], { cwd, input: JSON.stringify({ protocolVersion: 1, requestId: `owner-${sequence}`, document }), windowsHide: true, timeout: 120000,
      env: { ...process.env, PATH: `${path.resolve("../../.local/occt-build/win64/vc14/bin")};${path.resolve("src-tauri/resources/occt")};${process.env.PATH ?? ""}` } });
    const metrics = JSON.parse(fs.readFileSync(path.join(cwd, "result.json"), "utf8"));
    if (metrics.status !== "completed") throw new Error(JSON.stringify(metrics));
    return { metrics, glb: `data:model/gltf-binary;base64,${fs.readFileSync(path.join(cwd, "preview.glb")).toString("base64")}` };
  } };
}
function document() {
  const mm = (value: number) => ({ kind: "literal", mm: value });
  return { schemaVersion: 2, revisionId: "owners", parameters: [{ id: "width", name: "Width", valueMm: 40 }], features: [
    { id: "a_profile", name: "A profile", operation: { type: "rectangle", width: { kind: "parameter", parameterId: "width" }, depth: mm(20) } },
    { id: "a_pad", name: "A pad", operation: { type: "extrude", sketchId: "a_profile", distance: mm(10) } },
    { id: "a_rotate", name: "A rotate", operation: { type: "rotate", bodyFeatureId: "a_pad", axisOriginMm: [0, 0, 0], axisDirection: [0, 0, 1], angleDeg: 17 } },
    { id: "b_profile", name: "B profile", operation: { type: "rectangle", width: mm(12), depth: mm(16) } },
    { id: "b_pad", name: "B pad", operation: { type: "extrude", sketchId: "b_profile", distance: mm(9) } },
    { id: "b_move", name: "B move", operation: { type: "translate", bodyFeatureId: "b_pad", offsetMm: [65, 0, 0] } },
  ], bodies: [{ id: "body_a", name: "Body A", sourceFeatureId: "a_rotate" }, { id: "body_b", name: "Body B", sourceFeatureId: "b_move" }] };
}
for (const locale of ["ru", "en"] as const) test(`owner topology ${locale}: rotated edge click, fillet draft and independent reorder`, async ({ page }, info) => {
  const worker = workerFixture(); test.skip(!worker, "Requires the current local topology-enabled CAD worker");
  const initial = document(), built = worker!.build(initial), source = JSON.stringify(initial);
  const errors: string[] = []; page.on("pageerror", error => errors.push(error.message));
  await page.route("**/src/main.tsx", route => route.fulfill({ contentType: "application/javascript", body: "export {};" }));
  await page.addInitScript(locale => localStorage.setItem("forma.locale", locale), locale); await page.goto("/");
  await page.evaluate(async data => { const { mountTopologyHarness } = await import("/e2e/topologyHarness.tsx"); await mountTopologyHarness(data); }, { source, glb: built.glb, revision: initial.revisionId });
  const choose = async (role?: string) => {
    await expect.poll(() => page.evaluate(role => window.__topologyProbe?.candidates.find((candidate: any) => candidate.bodyId === "body_a" && candidate.reference && (!role || candidate.reference.role === role)) ?? null, role)).not.toBeNull();
    const target = await page.evaluate(role => window.__topologyProbe.candidates.find((candidate: any) => candidate.bodyId === "body_a" && candidate.reference && (!role || candidate.reference.role === role)), role);
    await page.mouse.click(target.x, target.y);
    await expect.poll(() => page.evaluate(() => window.__topologySelection?.topologyRef)).toEqual(target.reference);
    return target;
  };
  const selected = await choose();
  expect(selected.reference.ownerFeatureId).toBe("a_pad"); expect(selected.reference.occurrencePath).toEqual(["a_rotate"]);
  expect(selected.semanticKey).toBeNull(); // A rotated edge cannot use world-axis bounds as its identity.
  await page.getByTestId("open-editor").click();
  const editor = page.locator(".edge-fillet-editor"), add = editor.locator(".edge-fillet-controls button");
  await expect(add).toBeEnabled();
  const body = editor.locator(":scope > .editor-disclosure-body");
  const animationCount = () => body.evaluate(element => Number(element.getAttribute("data-animation-count")));
  await expect.poll(() => body.evaluate(element => element.getAnimations().length)).toBe(0);
  for (const theme of ["dark", "light"]) {
    await page.evaluate(theme => window.__topologyTheme(theme), theme);
    for (let pass = 0; pass < 2; pass++) {
      const before = await animationCount();
      await editor.locator("summary").click();
      await expect.poll(animationCount).toBe(before + 1);
      await expect(editor).not.toHaveAttribute("open", "");
      await editor.locator("summary").click();
      await expect.poll(animationCount).toBe(before + 2);
      await expect(editor).toHaveAttribute("open", "");
      const motion = await body.evaluate(element => element.getAnimations().map(animation => ({
        duration: animation.effect?.getTiming().duration,
        frames: (animation.effect as KeyframeEffect).getKeyframes().map(frame => frame.height),
      })));
      expect(motion).toHaveLength(1);
      expect(motion[0].duration).toBe(260);
      expect(motion[0].frames[0]).toBe("0px");
      expect(Number.parseFloat(motion[0].frames.at(-1)!)).toBeGreaterThan(32);
      await expect.poll(() => body.evaluate(element => element.getAnimations().length)).toBe(0);
    }
    await expect(editor.locator("summary svg.lucide-radius")).toHaveCount(1);
    await expect(add.locator("svg.lucide-plus")).toHaveCount(1);
    const input = editor.getByRole("spinbutton"), inputBox = (await input.boundingBox())!, buttonBox = (await add.boundingBox())!;
    expect(inputBox.height).toBe(32); expect(buttonBox.height).toBe(32);
    await page.locator(".model-editor-modal").screenshot({ path: info.outputPath(`owner-editor-${locale}-${theme}.png`) });
  }
  await editor.getByRole("spinbutton").fill("1.5"); await add.click();
  const draft = JSON.parse(await page.evaluate(() => window.__topologyDraft));
  const fillet = draft.features.at(-1);
  expect(fillet.operation.type).toBe("filletReferencedEdge"); expect(fillet.operation.reference).toEqual(selected.reference);
  expect(fillet.operation.bodyFeatureId).toBe("a_rotate"); expect(fillet.operation.radius.mm).toBe(1.5);
  expect(draft.bodies.find((body: any) => body.id === "body_b").sourceFeatureId).toBe("b_move");
  const filleted = worker!.build(draft); expect(filleted.metrics.volumeMm3).toBeGreaterThan(0);
  await page.evaluate(data => window.__topologyLoad(data), { source: JSON.stringify(draft), glb: filleted.glb, revision: "filleted" });
  await expect.poll(() => page.evaluate(() => window.__topologyProbe.catalogs.body_a?.length ?? 0)).toBeGreaterThan(0);
  await expect.poll(() => page.evaluate(() => window.__topologyProbe.catalogs.body_a?.every((edge: any) => !edge.topologyRef))).toBe(true);
  const changed = structuredClone(initial); changed.parameters[0].valueMm = 52; changed.revisionId = "resized";
  changed.features = [...changed.features.slice(3), ...changed.features.slice(0, 3)]; changed.bodies.reverse();
  const resized = worker!.build(changed);
  await page.evaluate(data => window.__topologyLoad(data), { source: JSON.stringify(changed), glb: resized.glb, revision: changed.revisionId });
  const sameRole = await choose(selected.reference.role); expect(sameRole.reference).toEqual(selected.reference);
  // The same authored role must still build on changed dimensions and a different independent feature order.
  const resizedDraft = structuredClone(changed) as any;
  resizedDraft.features.push(fillet);
  resizedDraft.bodies.find((body: any) => body.id === "body_a").sourceFeatureId = fillet.id;
  const resizedFillet = worker!.build(resizedDraft);
  expect(resizedFillet.metrics.volumeMm3).toBeLessThan(resized.metrics.volumeMm3);
  expect(resizedFillet.metrics.volumeMm3).toBeGreaterThan(filleted.metrics.volumeMm3);
  await page.getByTestId("open-editor").click(); await expect(add).toBeEnabled();
  // Wrong owner/occurrence belongs to the independent branch: UI cannot author that fillet.
  await page.evaluate(async () => { const { useWorkspace } = await import("/src/stores/workspace.ts"); const state = useWorkspace.getState(); state.setSelectedEdge({ ...state.selectedEdge!, topologyRef: { ...state.selectedEdge!.topologyRef!, ownerFeatureId: "b_pad", occurrencePath: ["b_move"] } }); });
  await expect(add).toBeDisabled();
  await page.emulateMedia({ reducedMotion: "reduce" });
  await editor.locator("summary").click();
  await expect(editor).not.toHaveAttribute("open", "");
  await editor.locator("summary").click();
  await expect(editor).toHaveAttribute("open", "");
  expect(await body.evaluate(element => element.getAnimations().length)).toBe(0);
  expect(errors).toEqual([]);
});
