import { useEffect, useRef, type RefObject } from "react";
import { useThree } from "@react-three/fiber";
import * as THREE from "three";
import { useWorkspace } from "../../../stores/workspace";
import { worldFrame } from "../frames";
import {
  beginTransform,
  cancelTransform,
  moveTransformPointer,
  setTransformAxis,
  setTransformInput,
  transformIntents,
  type TransformSession,
} from "../transformSession";
import { snapSessionIncrements } from "../../snapping/incrementSnap";
import { useTransformViewport } from "./transformStore";
import { bodyMeshes, PreviewMatrices, transformTarget } from "./sceneTargets";
import { cadWorldFrame } from "./renderBasis";
import { pointerSample, unwrapAngle, type ScreenPoint } from "./pointerSample";
import { modalKey } from "./modalKeys";
import type { OrbitHandle } from "../../viewer/CameraControl";

export interface ModalTransformHandle {
  begin: (
    kind: TransformSession["kind"],
  ) => "started" | "noSelection" | "unavailable";
}

/**
 * Renderer adapter for the independent transform session. It owns pointer and
 * key input only while a session is active and changes preview matrices, never
 * CAD data; confirmation is delegated to `onConfirm`.
 */
export function ModalTransform({
  object,
  interactive,
  controlsRef,
  handleRef,
  onConfirm,
}: {
  object: THREE.Group;
  interactive: boolean;
  controlsRef: RefObject<OrbitHandle | null>;
  handleRef: RefObject<ModalTransformHandle | null>;
  onConfirm: (
    session: TransformSession,
    sceneToken: string,
  ) => Promise<boolean>;
}) {
  const { camera, gl, size, invalidate } = useThree();
  const pointer = useRef<ScreenPoint>({
    x: size.width / 2,
    y: size.height / 2,
  });
  const active = useRef<{
    preview: PreviewMatrices;
    frame: THREE.Matrix4;
    start: ScreenPoint;
    angle: number;
    precise: boolean;
    snap: boolean;
  } | null>(null);
  const confirmRef = useRef(onConfirm);
  useEffect(() => {
    confirmRef.current = onConfirm;
  }, [onConfirm]);

  useEffect(() => {
    const element = gl.domElement;
    const store = useTransformViewport;
    const render = (session: TransformSession) => {
      const state = active.current;
      if (!state) return;
      const intents = transformIntents(
        snapSessionIncrements(session, state.snap),
      );
      if (intents) state.preview.apply(intents);
      invalidate();
    };
    const update = (session: TransformSession) => {
      store.getState().setSession(session);
      render(session);
    };
    const finish = (restore: boolean) => {
      const state = active.current;
      if (restore) state?.preview.restore();
      active.current = null;
      if (controlsRef.current) controlsRef.current.enabled = true;
      store.getState().setSession(null);
      invalidate();
    };
    const sample = (
      session: TransformSession,
      event?: { shiftKey: boolean; ctrlKey: boolean },
    ) => {
      const state = active.current;
      if (!state) return session;
      if (event) {
        state.precise = event.shiftKey;
        state.snap = store.getState().incrementSnap !== event.ctrlKey;
      }
      const frame =
        session.constraint?.space === "local"
          ? session.local
          : session.orientation;
      const reading = pointerSample({
        camera,
        size: { width: element.clientWidth, height: element.clientHeight },
        start: state.start,
        current: pointer.current,
        pivotMm: session.pivotMm,
        rotationAxisMm: frame[session.constraint?.axis ?? "z"],
        frame: state.frame,
      });
      if (!reading) return session;
      state.angle = unwrapAngle(state.angle, reading.angleDeg);
      return moveTransformPointer(
        session,
        { ...reading, angleDeg: state.angle },
        state.precise,
      );
    };
    const confirm = () => {
      const session = store.getState().session;
      const state = active.current;
      if (!session || !state) return;
      const snapped = snapSessionIncrements(session, state.snap);
      active.current = null;
      if (controlsRef.current) controlsRef.current.enabled = true;
      store.getState().setSession(null);
      // Preview stays until the committed revision replaces the scene; failures restore it.
      void confirmRef.current(snapped, object.uuid).then((committed) => {
        if (!committed) state.preview.restore();
        invalidate();
      });
    };
    handleRef.current = {
      begin: (kind) => {
        const workspace = useWorkspace.getState();
        const project = workspace.project;
        if (
          !interactive ||
          !project?.currentRevision ||
          store.getState().status.kind === "committing"
        )
          return "unavailable";
        const current = store.getState().session;
        if (current) {
          // Switching G/R/S keeps the targets and restarts motion from the current pointer.
          active.current!.preview.restore();
          active.current!.start = { ...pointer.current };
          active.current!.angle = 0;
          const restarted = beginTransform({
            capture: current.capture,
            kind,
            targets: current.targets,
            activeId: workspace.selected ?? current.targets[0].bodyId,
            pivotMode: current.pivotMode,
            cursorMm: store.getState().cursorMm,
            orientation: worldFrame,
            local: worldFrame,
          });
          if (restarted) update(restarted);
          return "started";
        }
        const ids = workspace.selectedBodies.length
          ? workspace.selectedBodies
          : workspace.selected
            ? [workspace.selected]
            : [];
        const meshes = bodyMeshes(object, ids).filter(
          (mesh) => mesh.visible && !mesh.userData.formaSelectionLocked,
        );
        const frame = cadWorldFrame(object);
        const targets = meshes.map((mesh) => transformTarget(mesh, frame));
        if (!meshes.length || targets.some((item) => !item))
          return "noSelection";
        const session = beginTransform({
          capture: {
            projectId: project.id,
            revisionId: project.currentRevision,
            sceneToken: object.uuid,
          },
          kind,
          targets: targets.map((item) => item!),
          activeId: workspace.selected ?? meshes[0].name,
          pivotMode: store.getState().pivotMode,
          cursorMm: store.getState().cursorMm,
          orientation: worldFrame,
          local: worldFrame,
        });
        if (!session) return "unavailable";
        active.current = {
          preview: new PreviewMatrices(meshes, frame),
          frame,
          start: { ...pointer.current },
          angle: 0,
          precise: false,
          snap: store.getState().incrementSnap,
        };
        if (controlsRef.current) controlsRef.current.enabled = false;
        store.getState().setStatus({ kind: "idle" });
        update(session);
        return "started";
      },
    };
    const move = (event: PointerEvent) => {
      const rect = element.getBoundingClientRect();
      pointer.current = {
        x: event.clientX - rect.left,
        y: event.clientY - rect.top,
      };
      const session = store.getState().session;
      if (session && active.current) update(sample(session, event));
    };
    const down = (event: PointerEvent) => {
      if (!active.current) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      // The click that ends the modal must not also change the selection.
      window.addEventListener("click", swallow, { capture: true, once: true });
      if (event.button === 0) confirm();
      else {
        const session = store.getState().session;
        if (session) cancelTransform(session);
        finish(true);
      }
    };
    const swallow = (event: MouseEvent) => {
      event.stopImmediatePropagation();
      event.preventDefault();
    };
    const menu = (event: MouseEvent) => {
      if (active.current) event.preventDefault();
    };
    const key = (event: KeyboardEvent) => {
      const session = store.getState().session;
      if (!session || !active.current) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.key === "Shift" || event.key === "Control") {
        update(sample(session, event));
        return;
      }
      const action = modalKey(session, event);
      if (action.type === "cancel") {
        cancelTransform(session);
        finish(true);
      } else if (action.type === "confirm") confirm();
      else if (action.type === "axis")
        update(sample(setTransformAxis(session, action.axis, action.exclude)));
      else if (action.type === "kind") handleRef.current?.begin(action.kind);
      else if (action.type === "input")
        update(setTransformInput(session, action.input));
    };
    const modifiers = (event: KeyboardEvent) => {
      const session = store.getState().session;
      if (!session || !active.current) return;
      if (event.key === "Shift" || event.key === "Control")
        update(sample(session, event));
    };
    const blur = () => {
      if (active.current) finish(true);
    };
    window.addEventListener("pointermove", move, true);
    element.addEventListener("pointerdown", down, true);
    element.addEventListener("contextmenu", menu, true);
    window.addEventListener("keydown", key, true);
    window.addEventListener("keyup", modifiers, true);
    window.addEventListener("blur", blur);
    return () => {
      window.removeEventListener("pointermove", move, true);
      element.removeEventListener("pointerdown", down, true);
      element.removeEventListener("contextmenu", menu, true);
      window.removeEventListener("keydown", key, true);
      window.removeEventListener("keyup", modifiers, true);
      window.removeEventListener("blur", blur);
      handleRef.current = null;
      // A scene change or unmount cancels an unconfirmed preview.
      if (active.current) finish(true);
    };
  }, [camera, gl, object, interactive, controlsRef, handleRef, invalidate]);
  return null;
}
