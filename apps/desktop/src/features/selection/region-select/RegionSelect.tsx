import { useEffect } from "react";
import { useThree } from "@react-three/fiber";
import { create } from "zustand";
import type * as THREE from "three";
import { t } from "../../../i18n";
import { useWorkspace } from "../../../stores/workspace";
import { boxRegion, type Point2, type SelectionRegion } from "./regionGeometry";
import { regionSelection } from "./regionSelection";
import { projectBodies } from "./projectBodies";
import { visibleRegionHits } from "./occlusion";
import { updateSelection, type SelectionAction } from "../selectionModel";
import type { OrbitHandle } from "../../viewer/CameraControl";
import "./regionSelect.css";

type Mode = "box" | "circle" | null;
interface RegionSelectState {
  mode: Mode;
  /** Client coordinates: the overlay is fixed-position. */
  start: Point2 | null;
  end: Point2 | null;
  lasso: Point2[] | null;
  brush: { center: Point2; radius: number } | null;
  setMode: (mode: Mode) => void;
  set: (patch: Partial<Omit<RegionSelectState, "set" | "setMode">>) => void;
}
export const useRegionSelect = create<RegionSelectState>((set) => ({
  mode: null,
  start: null,
  end: null,
  lasso: null,
  brush: null,
  setMode: (mode) =>
    set({ mode, start: null, end: null, lasso: null, brush: null }),
  set: (patch) => set(patch),
}));

let brushRadius = 30;

/**
 * Region selection of bodies. B: one box gesture. C: a brush that paints while
 * the left button is held (Shift removes, wheel resizes, Esc/RMB/Enter exits).
 * Ctrl+RMB drag: lasso (Ctrl+Shift removes). Outside X-ray only bodies that
 * are actually visible inside the region are picked.
 */
export function RegionSelectController({
  object,
  controlsRef,
  enabled,
  through,
}: {
  object: THREE.Group;
  controlsRef: React.RefObject<OrbitHandle | null>;
  enabled: boolean;
  through: boolean;
}) {
  const { camera, gl } = useThree();
  const mode = useRegionSelect((state) => state.mode);
  useEffect(() => {
    if (!enabled) return;
    const element = gl.domElement;
    const store = useRegionSelect.getState;
    const rect = () => element.getBoundingClientRect();
    const local = ([x, y]: Point2): Point2 => [x - rect().left, y - rect().top];
    const apply = (region: SelectionRegion, action: SelectionAction) => {
      const workspace = useWorkspace.getState();
      const project = workspace.project;
      if (!project?.currentRevision) return;
      const size = { width: element.clientWidth, height: element.clientHeight };
      const projected = projectBodies(object, camera, size);
      let hits = regionSelection(region, projected, "body", true);
      if (!through)
        hits = visibleRegionHits({ object, camera, size, region, hits });
      const scope = {
        projectId: project.id,
        revisionId: project.currentRevision,
        sceneToken: object.uuid,
      };
      const next = updateSelection(
        { scope, keys: workspace.selectedBodies, active: workspace.selected },
        scope,
        projected.map((item) => item.target),
        hits,
        action,
        "body",
      );
      workspace.setBodySelection(next.keys, next.active);
    };
    const swallowClick = () =>
      window.addEventListener(
        "click",
        (event) => event.stopImmediatePropagation(),
        {
          capture: true,
          once: true,
        },
      );
    const menu = (event: MouseEvent) => {
      if (store().lasso || store().mode) event.preventDefault();
    };
    let painting: SelectionAction | null = null;
    const brushAt = (event: PointerEvent | WheelEvent): Point2 => [
      event.clientX,
      event.clientY,
    ];
    const paint = (center: Point2) => {
      if (!painting) return;
      apply(
        { kind: "circle", center: local(center), radius: brushRadius },
        painting,
      );
    };
    const down = (event: PointerEvent) => {
      const { mode } = store();
      // Lasso works in any body-selection state.
      if (event.button === 2 && event.ctrlKey && !mode) {
        event.preventDefault();
        event.stopImmediatePropagation();
        if (controlsRef.current) controlsRef.current.enabled = false;
        store().set({ lasso: [[event.clientX, event.clientY]] });
        return;
      }
      if (!mode) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.button !== 0) return store().setMode(null);
      if (mode === "box") {
        const point: Point2 = [event.clientX, event.clientY];
        store().set({ start: point, end: point });
      } else {
        painting = event.shiftKey ? "subtract" : "add";
        paint(brushAt(event));
      }
    };
    const move = (event: PointerEvent) => {
      const state = store();
      if (state.lasso) {
        const last = state.lasso[state.lasso.length - 1];
        if (
          Math.hypot(event.clientX - last[0], event.clientY - last[1]) > 4 &&
          state.lasso.length < 4096
        )
          store().set({
            lasso: [...state.lasso, [event.clientX, event.clientY]],
          });
        return;
      }
      if (state.mode === "box" && state.start)
        store().set({ end: [event.clientX, event.clientY] });
      if (state.mode === "circle") {
        store().set({ brush: { center: brushAt(event), radius: brushRadius } });
        paint(brushAt(event));
      }
    };
    const up = (event: PointerEvent) => {
      const state = store();
      if (state.lasso) {
        const points = state.lasso;
        store().set({ lasso: null });
        if (controlsRef.current) controlsRef.current.enabled = true;
        if (points.length >= 3) {
          apply(
            { kind: "polygon", points: points.map(local) },
            event.shiftKey ? "subtract" : "add",
          );
          swallowClick();
        }
        return;
      }
      if (state.mode === "box" && state.start) {
        apply(
          boxRegion(local(state.start), local([event.clientX, event.clientY])),
          event.shiftKey ? "subtract" : "add",
        );
        swallowClick();
        store().setMode(null);
      }
      if (state.mode === "circle") {
        painting = null;
        swallowClick();
      }
    };
    const wheel = (event: WheelEvent) => {
      if (store().mode !== "circle") return;
      event.preventDefault();
      event.stopImmediatePropagation();
      brushRadius = Math.min(
        200,
        Math.max(6, brushRadius * (event.deltaY > 0 ? 0.9 : 1.1)),
      );
      store().set({ brush: { center: brushAt(event), radius: brushRadius } });
    };
    const key = (event: KeyboardEvent) => {
      if (!store().mode) return;
      if (["Escape", "Enter", "NumpadEnter"].includes(event.code)) {
        event.preventDefault();
        event.stopImmediatePropagation();
        store().setMode(null);
      }
    };
    element.addEventListener("pointerdown", down, true);
    element.addEventListener("contextmenu", menu, true);
    element.addEventListener("wheel", wheel, { capture: true, passive: false });
    window.addEventListener("pointermove", move, true);
    window.addEventListener("pointerup", up, true);
    window.addEventListener("keydown", key, true);
    return () => {
      element.removeEventListener("pointerdown", down, true);
      element.removeEventListener("contextmenu", menu, true);
      element.removeEventListener("wheel", wheel, true);
      window.removeEventListener("pointermove", move, true);
      window.removeEventListener("pointerup", up, true);
      window.removeEventListener("keydown", key, true);
    };
  }, [enabled, through, camera, gl, object, controlsRef]);
  // Orbit is paused while a box or brush gesture owns the pointer.
  useEffect(() => {
    if (!mode) return;
    const controls = controlsRef.current;
    if (controls) controls.enabled = false;
    return () => {
      if (controls) controls.enabled = true;
    };
  }, [mode, controlsRef]);
  return null;
}

export function RegionSelectOverlay() {
  const { mode, start, end, lasso, brush } = useRegionSelect();
  if (lasso && lasso.length > 1)
    return (
      <svg className="region-select-svg" aria-hidden>
        <polygon points={lasso.map(([x, y]) => `${x},${y}`).join(" ")} />
      </svg>
    );
  if (mode === "circle")
    return (
      <>
        <div className="region-select-hint" role="status">
          {t(
            "Paint to add bodies · Shift removes · wheel changes size · Esc finishes",
          )}
        </div>
        {brush && (
          <div
            className="region-select-brush"
            style={{
              left: brush.center[0] - brush.radius,
              top: brush.center[1] - brush.radius,
              width: brush.radius * 2,
              height: brush.radius * 2,
            }}
          />
        )}
      </>
    );
  if (mode !== "box") return null;
  if (!start || !end)
    return (
      <div className="region-select-hint" role="status">
        {t(
          "Drag to add bodies to the selection · Shift subtracts · Esc cancels",
        )}
      </div>
    );
  return (
    <div
      className="region-select-rect"
      style={{
        left: Math.min(start[0], end[0]),
        top: Math.min(start[1], end[1]),
        width: Math.abs(end[0] - start[0]),
        height: Math.abs(end[1] - start[1]),
      }}
    />
  );
}
