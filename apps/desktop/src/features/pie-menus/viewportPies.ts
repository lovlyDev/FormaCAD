import {useWorkspace} from "../../stores/workspace";
import { t } from "../../i18n";
import type { RenderMode } from "../viewer/renderMode";
import {
  pivotModes,
  useTransformViewport,
} from "../transform/viewport/transformStore";
import { useSnapSettings } from "../snapping/cursor/snapSettings";
import {edgeSnapActions} from "./edgeSnapActions";
import {beginSnapRequest,captureSnapSource,captureDisplayedSnapSource} from "../snapping/snapRequest";
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

export function snapPie(object?:{uuid:string}) {
  const { incrementSnap, setIncrementSnap, setCursor } =
    useTransformViewport.getState();
  const { exact, setExact } = useSnapSettings.getState();
  const menuSource=captureSnapSource(),displayed=object?captureDisplayedSnapSource(object):menuSource;
  const menuCurrent=()=>(menuSource?menuSource.isCurrent():!useWorkspace.getState().project)&&(!object||!!displayed?.isCurrent());
  const edgeSnap=edgeSnapActions(object);
  const edgeReady=edgeSnap.ready;
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
        run: () => edgeSnap.run("edgeMidpoint"),
      },
      {
        id: "circle-center",
        label: t("Cursor to circle center"),
        disabled: !edgeReady,
        run: () => edgeSnap.run("circleCenter"),
      },
      {
        id: "origin",
        label: t("Cursor to world origin"),
        run: () => {if(!menuCurrent())return;const request=beginSnapRequest(menuSource);setCursor([0,0,0]);request.finish();},
      },
    ] satisfies PieItem[],
  };
}
