import { useEffect } from "react";
import { useThree } from "@react-three/fiber";
import { create } from "zustand";
import type * as THREE from "three";
import { t } from "../../../i18n";
import { useWorkspace } from "../../../stores/workspace";
import { boxRegion, type Point2 } from "./regionGeometry";
import { regionSelection } from "./regionSelection";
import { projectBodies } from "./projectBodies";
import { updateSelection, type SelectionAction } from "../selectionModel";
import type { OrbitHandle } from "../../viewer/CameraControl";
import "./boxSelect.css";

interface BoxSelectState {
  active: boolean;
  start: Point2 | null;
  end: Point2 | null;
  setActive: (active: boolean) => void;
  setRect: (start: Point2 | null, end: Point2 | null) => void;
}
export const useBoxSelect = create<BoxSelectState>((set) => ({
  active: false,
  start: null,
  end: null,
  setActive: (active) => set({ active, start: null, end: null }),
  setRect: (start, end) => set({ start, end }),
}));

/** B starts one box gesture: drag adds, Shift+drag subtracts, Esc or RMB cancels. */
export function BoxSelectController({
  object,
  controlsRef,
}: {
  object: THREE.Group;
  controlsRef: React.RefObject<OrbitHandle | null>;
}) {
  const { camera, gl } = useThree();
  const active = useBoxSelect((state) => state.active);
  useEffect(() => {
    if (!active) return;
    const element = gl.domElement;
    const store = useBoxSelect.getState;
    if (controlsRef.current) controlsRef.current.enabled = false;
    // The overlay is fixed-position, so the store keeps client coordinates.
    const client = (event: PointerEvent): Point2 => [
      event.clientX,
      event.clientY,
    ];
    const local = (point: Point2): Point2 => {
      const rect = element.getBoundingClientRect();
      return [point[0] - rect.left, point[1] - rect.top];
    };
    const stop = () => {
      store().setActive(false);
    };
    const down = (event: PointerEvent) => {
      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.button !== 0) return stop();
      const point = client(event);
      store().setRect(point, point);
    };
    const move = (event: PointerEvent) => {
      const { start } = store();
      if (start) store().setRect(start, client(event));
    };
    const up = (event: PointerEvent) => {
      const { start } = store();
      if (!start) return;
      const end = client(event);
      const workspace = useWorkspace.getState();
      const project = workspace.project;
      if (project?.currentRevision) {
        const projected = projectBodies(object, camera, {
          width: element.clientWidth,
          height: element.clientHeight,
        });
        const hits = regionSelection(
          boxRegion(local(start), local(end)),
          projected,
          "body",
          true,
        );
        const scope = {
          projectId: project.id,
          revisionId: project.currentRevision,
          sceneToken: object.uuid,
        };
        const action: SelectionAction = event.shiftKey ? "subtract" : "add";
        const next = updateSelection(
          { scope, keys: workspace.selectedBodies, active: workspace.selected },
          scope,
          projected.map((item) => item.target),
          hits,
          action,
          "body",
        );
        workspace.setBodySelection(next.keys, next.active);
      }
      window.addEventListener("click", swallow, { capture: true, once: true });
      stop();
    };
    const swallow = (event: MouseEvent) => event.stopImmediatePropagation();
    const key = (event: KeyboardEvent) => {
      if (event.code !== "Escape") return;
      event.preventDefault();
      event.stopImmediatePropagation();
      stop();
    };
    element.addEventListener("pointerdown", down, true);
    window.addEventListener("pointermove", move, true);
    window.addEventListener("pointerup", up, true);
    window.addEventListener("keydown", key, true);
    return () => {
      element.removeEventListener("pointerdown", down, true);
      window.removeEventListener("pointermove", move, true);
      window.removeEventListener("pointerup", up, true);
      window.removeEventListener("keydown", key, true);
      if (controlsRef.current) controlsRef.current.enabled = true;
    };
  }, [active, camera, gl, object, controlsRef]);
  return null;
}

export function BoxSelectOverlay() {
  const { active, start, end } = useBoxSelect();
  if (!active) return null;
  if (!start || !end)
    return (
      <div className="box-select-hint" role="status">
        {t(
          "Drag to add bodies to the selection · Shift subtracts · Esc cancels",
        )}
      </div>
    );
  return (
    <div
      className="box-select-rect"
      style={{
        left: Math.min(start[0], end[0]),
        top: Math.min(start[1], end[1]),
        width: Math.abs(end[0] - start[0]),
        height: Math.abs(end[1] - start[1]),
      }}
    />
  );
}
