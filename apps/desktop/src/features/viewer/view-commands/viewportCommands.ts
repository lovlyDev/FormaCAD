import type { Dispatch, RefObject, SetStateAction } from "react";
import * as THREE from "three";
import { t } from "../../../i18n";
import { useWorkspace } from "../../../stores/workspace";
import {
  updateSelection,
  type SelectionAction,
} from "../../selection/selectionModel";
import { useBoxSelect } from "../../selection/region-select/BoxSelect";
import type { VisibilityAction } from "../../visibility/visibilityModel";
import type { ModalTransformHandle } from "../../transform/viewport/ModalTransform";
import { useTransformViewport } from "../../transform/viewport/transformStore";
import type { ViewportCommands } from "../../keymap/useViewportKeymap";
import type { RenderMode } from "../renderMode";
import type { SelectionFilter } from "../selectionFilter";
import type { StandardView } from "./standardViews";
import { usePieMenu, type PieItem } from "../../pie-menus/PieMenu";
import { pivotPie, shadingPie, snapPie } from "../../pie-menus/viewportPies";

export interface ViewportCommandContext {
  object: THREE.Group;
  interactive: boolean;
  transformRef: RefObject<ModalTransformHandle | null>;
  pointer: RefObject<{ x: number; y: number }>;
  mode: RenderMode;
  setMode: (mode: RenderMode) => void;
  setView: (view: StandardView) => void;
  setOrtho: Dispatch<SetStateAction<boolean>>;
  fitAll: () => void;
  fitSelection: () => void;
  setToolbarOpen: Dispatch<SetStateAction<boolean>>;
  setSidebarOpen: Dispatch<SetStateAction<boolean>>;
  setSelectionMode: (mode: SelectionFilter) => void;
  hasCadFaces: boolean;
  hasCadEdges: boolean;
  onVisibility?: (action: VisibilityAction) => void;
}

function selectBodies(object: THREE.Group, action: SelectionAction) {
  const workspace = useWorkspace.getState();
  const project = workspace.project;
  if (!project?.currentRevision) return;
  const universe: {
    key: string;
    bodyId: string;
    kind: "body";
    selectable: boolean;
    visible: boolean;
  }[] = [];
  object.traverse((node) => {
    if (
      node instanceof THREE.Mesh &&
      !node.userData.formaSelectionOverlay &&
      node.name
    )
      universe.push({
        key: node.name,
        bodyId: node.name,
        kind: "body",
        selectable: !node.userData.formaSelectionLocked,
        visible: node.visible,
      });
  });
  const scope = {
    projectId: project.id,
    revisionId: project.currentRevision,
    sceneToken: object.uuid,
  };
  const next = updateSelection(
    { scope, keys: workspace.selectedBodies, active: workspace.selected },
    scope,
    universe,
    [],
    action,
    "body",
  );
  workspace.setBodySelection(next.keys, next.active);
}

const transformErrors = {
  noSelection: "Select a visible, unlocked body to transform.",
  unavailable:
    "Transforms need a committed native CAD model and an idle viewport.",
} as const;

/** Maps registry command ids to viewport actions; absent handlers are unavailable shortcuts. */
export function viewportCommands(
  context: ViewportCommandContext,
): ViewportCommands {
  const begin = (kind: "move" | "rotate" | "scale") => () => {
    const result = context.transformRef.current?.begin(kind) ?? "unavailable";
    if (result !== "started")
      useTransformViewport
        .getState()
        .setStatus({ kind: "error", message: t(transformErrors[result]) });
  };
  const pie = (build: () => { title: string; items: PieItem[] }) => () => {
    const { title, items } = build();
    const point = context.pointer.current ?? {
      x: window.innerWidth / 2,
      y: window.innerHeight / 2,
    };
    usePieMenu.getState().show(title, items, point.x, point.y);
  };
  const visibility = (action: VisibilityAction) =>
    context.onVisibility ? () => context.onVisibility!(action) : undefined;
  const commands: ViewportCommands = {
    "view.front": () => context.setView("front"),
    "view.back": () => context.setView("back"),
    "view.right": () => context.setView("right"),
    "view.left": () => context.setView("left"),
    "view.top": () => context.setView("top"),
    "view.bottom": () => context.setView("bottom"),
    "view.projection": () => context.setOrtho((value) => !value),
    "view.fitAll": context.fitAll,
    "view.fitSelection": context.fitSelection,
    "view.toolbar": () => context.setToolbarOpen((value) => !value),
    "view.sidebar": () => context.setSidebarOpen((value) => !value),
    "view.xray": () =>
      context.setMode(context.mode === "xray" ? "edges" : "xray"),
    "view.shadingPie": pie(() => shadingPie(context.mode, context.setMode)),
  };
  if (!context.interactive) return commands;
  return {
    ...commands,
    "transform.move": begin("move"),
    "transform.rotate": begin("rotate"),
    "transform.scale": begin("scale"),
    "transform.adjustLast": () =>
      useTransformViewport.getState().setAdjustOpen(true),
    "transform.pivotPie": pie(pivotPie),
    "snap.pie": pie(snapPie),
    "selection.all": () => selectBodies(context.object, "all"),
    "selection.none": () => selectBodies(context.object, "none"),
    "selection.invert": () => selectBodies(context.object, "invert"),
    "selection.box": () => useBoxSelect.getState().setActive(true),
    "selection.body": () => context.setSelectionMode("body"),
    "selection.face": context.hasCadFaces
      ? () => context.setSelectionMode("face")
      : undefined,
    "selection.edge": context.hasCadEdges
      ? () => context.setSelectionMode("edge")
      : undefined,
    "selection.vertex": context.hasCadEdges
      ? () => context.setSelectionMode("vertex")
      : undefined,
    "visibility.hide": visibility("hide"),
    "visibility.hideOthers": visibility("hideOthers"),
    "visibility.showAll": visibility("showAll"),
    "visibility.isolate": visibility("isolate"),
  };
}
