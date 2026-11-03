import { useEffect, useMemo } from "react";
import { useThree } from "@react-three/fiber";
import { create } from "zustand";
import * as THREE from "three";
import { number, t } from "../../../i18n";
import { useWorkspace } from "../../../stores/workspace";
import type { Vec3 } from "../../transform/frames";
import {
  cadPointToRender,
  cadWorldFrame,
} from "../../transform/viewport/renderBasis";
import { findSnapCandidate } from "../../snapping/snapCandidate";
import { resolveSnap } from "../../snapping/resolveSnap";
import "./vertexSelect.css";

interface VertexSelectionState {
  selection: {
    bodyId: string;
    revisionId: string;
    pointMm: Vec3;
    exact: boolean;
  } | null;
  set: (selection: VertexSelectionState["selection"]) => void;
}
export const useVertexSelection = create<VertexSelectionState>((set) => ({
  selection: null,
  set: (selection) => set({ selection }),
}));

/**
 * Vertex filter of the selection: a click picks the nearest BREP vertex of the
 * preview catalogue; a referenced vertex is then re-read exactly from OCCT.
 */
export function VertexSelectController({
  object,
  enabled,
  light,
}: {
  object: THREE.Group;
  enabled: boolean;
  light: boolean;
}) {
  const { camera, gl, invalidate } = useThree();
  const selection = useVertexSelection((state) => state.selection);
  const revisionId = useWorkspace((state) => state.project?.currentRevision);
  const visible = selection && selection.revisionId === revisionId;
  const position = useMemo(
    () =>
      selection
        ? cadPointToRender(selection.pointMm, cadWorldFrame(object))
        : null,
    [selection, object],
  );
  useEffect(() => {
    if (!enabled) return;
    const element = gl.domElement;
    const down = (event: PointerEvent) => {
      if (event.button !== 0 || event.shiftKey || event.altKey) return;
      const rect = element.getBoundingClientRect();
      const candidate = findSnapCandidate({
        object,
        camera,
        screen: { x: event.clientX - rect.left, y: event.clientY - rect.top },
        size: { width: rect.width, height: rect.height },
        frame: cadWorldFrame(object),
        surfaces: false,
      });
      const project = useWorkspace.getState().project;
      if (!candidate || !project?.currentRevision) {
        useVertexSelection.getState().set(null);
        return;
      }
      const revision = project.currentRevision;
      useWorkspace.getState().setSelected(candidate.bodyId);
      useVertexSelection.getState().set({
        bodyId: candidate.bodyId,
        revisionId: revision,
        pointMm: candidate.pointMm,
        exact: false,
      });
      invalidate();
      void resolveSnap(candidate).then((resolved) => {
        const current = useVertexSelection.getState().selection;
        if (
          !resolved ||
          current?.revisionId !== revision ||
          current.bodyId !== candidate.bodyId
        )
          return;
        useVertexSelection.getState().set({ ...current, ...resolved });
        invalidate();
      });
    };
    element.addEventListener("pointerdown", down);
    return () => element.removeEventListener("pointerdown", down);
  }, [enabled, camera, gl, object, invalidate]);
  if (!visible || !position) return null;
  return (
    <mesh position={position} renderOrder={31}>
      <sphereGeometry args={[1.2, 16, 12]} />
      <meshBasicMaterial
        color={light ? "#7c3aed" : "#c4b5fd"}
        depthTest={false}
      />
    </mesh>
  );
}

export function VertexSelectionInfo() {
  const selection = useVertexSelection((state) => state.selection);
  const revisionId = useWorkspace((state) => state.project?.currentRevision);
  if (!selection || selection.revisionId !== revisionId) return null;
  return (
    <div className="vertex-selection-info" role="status">
      <strong>{t("Vertex")}</strong>
      <span>
        {selection.pointMm.map((value) => number(value, 4)).join(" · ")}{" "}
        {t("mm")}
      </span>
      <small>
        {selection.exact ? t("exact (OCCT)") : t("approximate (mesh)")}
      </small>
    </div>
  );
}
