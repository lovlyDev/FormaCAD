import { expect, test, type Page } from "@playwright/test";
import { writeFile } from "node:fs/promises";

/** Observe one genuine click; never set selections or retry a missed click. */
export async function pickPairViewport(page: Page, type: "edges" | "faces", role: string) {
  const info = test.info();
  await expect.poll(() => page.evaluate(({ type, role }) =>
    (window as any).__applyHarness.candidates()?.[type]
      .find((item: any) => item.reference?.role === role) ?? null,
  { type, role })).not.toBeNull();

  const target = await page.evaluate(async ({ type, role }) => {
    const { useWorkspace } = await import("/src/stores/workspace.ts");
    const globals = window as any, harness = globals.__applyHarness;
    const canvas = document.querySelector(".viewer-area canvas") as HTMLCanvasElement;
    const candidate = harness.candidates()[type].find((item: any) => item.reference?.role === role);
    const describe = (element: Element | null) => element ? {
      tag: element.tagName, className: String(element.className),
    } : null;
    const snapshot = () => {
      const state = useWorkspace.getState(), rect = canvas.getBoundingClientRect();
      const inspection = harness.inspect();
      const current = harness.candidates()?.[type].find((item: any) => item.reference?.role === role);
      return { now: performance.now(), camera: inspection?.camera, bodyUuid: inspection?.uuid,
        canvasRect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
        scenePhase: canvas.closest(".viewer-area")?.getAttribute("data-scene-phase"),
        // Viewer owns mode locally; record its real active toolbar controls.
        activeToolbar: [...document.querySelectorAll(".view-toolbar button.active")]
          .map(button => button.getAttribute("aria-label")), selectedBody: state.selected,
        selectedEdge: state.selectedEdge, selectedFace: state.selectedFace,
        candidate: current ?? null };
    };
    globals.__pairViewportPickProbe?.cleanup?.();
    const events: unknown[] = [];
    const listener = (event: MouseEvent) => {
      events.push({ event: event.type, point: { x: event.clientX, y: event.clientY },
        eventTarget: describe(event.target as Element),
        hitElement: describe(document.elementFromPoint(event.clientX, event.clientY)),
        snapshot: snapshot() });
      if (event.type === "click") queueMicrotask(() => events.push({ event: "click:microtask", snapshot: snapshot() }));
    };
    for (const name of ["pointerdown", "pointerup", "click"])
      canvas.addEventListener(name, listener as EventListener, true);
    globals.__pairViewportPickProbe = {
      type, role, expected: candidate, created: snapshot(), events,
      snapshot, cleanup: () => {
        for (const name of ["pointerdown", "pointerup", "click"])
          canvas.removeEventListener(name, listener as EventListener, true);
      },
    };
    return { ...candidate, created: globals.__pairViewportPickProbe.created };
  }, { type, role });

  console.log("PAIR_PICK_DIAGNOSTIC " + JSON.stringify({ stage: "before-click", type, role, target }));
  try {
    await page.mouse.click(target.x, target.y);
    await expect.poll(() => page.evaluate(type => type === "edges"
      ? (window as any).__applyHarness.selectedEdge()?.topologyRef
      : (window as any).__applyHarness.selectedFace()?.topologyRef, type)).toEqual(target.reference);
  } finally {
    const probe = await page.evaluate(() => {
      const probe = (window as any).__pairViewportPickProbe;
      if (!probe) return null;
      const result = { type: probe.type, role: probe.role, expected: probe.expected,
        created: probe.created, events: probe.events, after: probe.snapshot() };
      probe.cleanup();
      return result;
    }).catch(error => ({ error: String(error), type, role, target }));
    const json = JSON.stringify(probe);
    // Persist beside screenshots even with the list reporter (attachments alone
    // are not necessarily emitted to disk by that reporter).
    const path = info.outputPath(`pair-pick-${type}-${role.replace(/[^a-z0-9_-]/gi, "_")}.json`);
    await writeFile(path, json);
    await info.attach("actual-pair-single-click", { path, contentType: "application/json" });
    console.log("PAIR_PICK_DIAGNOSTIC " + JSON.stringify({ stage: "after-click", probe }));
  }
}
