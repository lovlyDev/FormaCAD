import { t } from "../../i18n";
import type { RenderMode } from "../viewer/renderMode";
import {
  pivotModes,
  useTransformViewport,
} from "../transform/viewport/transformStore";
import { useSnapSettings } from "../snapping/cursor/snapSettings";
import { useWorkspace } from "../../stores/workspace";
import { exactSnap } from "../snapping/snapApi";
import type { PieItem } from "./PieMenu";

const shading: { mode: RenderMode; label: string }[] = [
  { mode: "edges", label: "Solid + edges" },
  { mode: "solid", label: "Solid" },
  { mode: "wireframe", label: "Wireframe" },
  { mode: "transparent", label: "Transparent" },
  { mode: "xray", label: "X-Ray" },
  { mode: "ghost", label: "Ghost" },
];

export function shadingPie(
  current: RenderMode,
  setMode: (mode: RenderMode) => void,
) {
  return {
    title: t("Shading"),
    items: shading.map((item): PieItem => ({
      id: item.mode,
      label: t(item.label),
      active: item.mode === current,
      run: () => setMode(item.mode),
    })),
  };
}

const pivotLabels = {
  bounds: "Bounding box center",
  median: "Median point",
  cursor: "3D cursor",
  active: "Active element",
  individual: "Individual origins",
} as const;

export function pivotPie() {
  const store = useTransformViewport.getState();
  return {
    title: t("Pivot point"),
    items: pivotModes.map((mode): PieItem => ({
      id: mode,
      label: t(pivotLabels[mode]),
      active: store.pivotMode === mode,
      run: () => useTransformViewport.getState().setPivotMode(mode),
    })),
  };
}

/** Cursor to a selected CAD edge, computed by OCCT on the committed body. */
function cursorToEdge(kind: "edgeMidpoint" | "circleCenter") {
  const { project, selectedEdge } = useWorkspace.getState();
  const reference = selectedEdge?.topologyRef;
  if (!project?.currentRevision || !selectedEdge || !reference) return;
  const revisionId = project.currentRevision;
  const snap = useSnapSettings.getState();
  snap.setPending(true);
  void exactSnap(
    { projectId: project.id, revisionId, bodyId: selectedEdge.bodyId },
    { kind, reference },
  )
    .then((result) => {
      if (useWorkspace.getState().project?.currentRevision === revisionId) {
        useTransformViewport.getState().setCursor(result.pointMm);
        useSnapSettings.getState().setError(null);
      }
    })
    .catch((cause) => useSnapSettings.getState().setError(cause))
    .finally(() => useSnapSettings.getState().setPending(false));
}

export function snapPie() {
  const { incrementSnap, setIncrementSnap, setCursor } =
    useTransformViewport.getState();
  const { exact, setExact } = useSnapSettings.getState();
  const { selectedEdge, project } = useWorkspace.getState();
  const edgeReady =
    !!selectedEdge?.topologyRef &&
    selectedEdge.revisionId === project?.currentRevision;
  return {
    title: t("Snapping"),
    items: [
      {
        id: "increment",
        label: t("Increment snapping"),
        active: incrementSnap,
        run: () => setIncrementSnap(!incrementSnap),
      },
      {
        id: "exact",
        label: t("Exact surface snapping for the 3D cursor"),
        active: exact,
        run: () => setExact(!exact),
      },
      {
        id: "edge-midpoint",
        label: t("Cursor to edge midpoint"),
        disabled: !edgeReady,
        run: () => cursorToEdge("edgeMidpoint"),
      },
      {
        id: "circle-center",
        label: t("Cursor to circle center"),
        disabled: !edgeReady,
        run: () => cursorToEdge("circleCenter"),
      },
      {
        id: "origin",
        label: t("Cursor to world origin"),
        run: () => setCursor([0, 0, 0]),
      },
    ] satisfies PieItem[],
  };
}
