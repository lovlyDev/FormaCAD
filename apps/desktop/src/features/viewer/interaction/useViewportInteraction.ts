import { useCallback, useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import * as THREE from "three";
import { errorText, t } from "../../../i18n";
import { useWorkspace } from "../../../stores/workspace";
import type { Project } from "../../../types";
import { useViewportKeymap } from "../../keymap/useViewportKeymap";
import type { ModalTransformHandle } from "../../transform/viewport/ModalTransform";
import {
  commitTransform,
  transformName,
} from "../../transform/viewport/commitTransform";
import { useTransformViewport } from "../../transform/viewport/transformStore";
import {
  transformIntents,
  type TransformSession,
} from "../../transform/transformSession";
import { intentIsNoop } from "../../transform/transformPreview";
import { useSnapSettings } from "../../snapping/cursor/snapSettings";
import { bodyMeshes } from "../../transform/viewport/sceneTargets";
import {
  viewportCommands,
  type ViewportCommandContext,
} from "../view-commands/viewportCommands";

type InteractionOptions = Omit<
  ViewportCommandContext,
  "transformRef" | "pointer" | "fitSelection"
> & {
  enabled: boolean;
  emulateNumpad: boolean;
  fitBox: (box: THREE.Box3 | null) => void;
  confirmEdit?: ConfirmEdit;
};

/** App-level confirmation (permission dialog when settings require it); resolves to whether run committed. */
export type ConfirmEdit = (
  title: string,
  description: string,
  detail: string,
  run: () => Promise<boolean>,
) => Promise<boolean>;

/** Adopts a host-committed project the same way the assistant apply route does. */
export function useAdoptProject() {
  const query = useQueryClient();
  return useCallback(
    (project: Project) => {
      query.setQueryData<Project[]>(["projects"], (current) =>
        (current ?? []).some((item) => item.id === project.id)
          ? current!.map((item) => (item.id === project.id ? project : item))
          : [...(current ?? []), project],
      );
      if (useWorkspace.getState().project?.id === project.id)
        useWorkspace.getState().setProject(project);
      void query.invalidateQueries({ queryKey: ["model-history", project.id] });
    },
    [query],
  );
}

/**
 * M2 viewport wiring: one keymap route, modal transform confirmation through
 * the Command API, F9 state and the pie/selection commands.
 */
export function useViewportInteraction(options: InteractionOptions) {
  const transformRef = useRef<ModalTransformHandle | null>(null);
  const pointer = useRef({
    x: window.innerWidth / 2,
    y: window.innerHeight / 2,
  });
  const adopt = useAdoptProject();
  useEffect(() => {
    useTransformViewport.getState().hydrate();
    useSnapSettings.getState().hydrate();
    const move = (event: PointerEvent) => {
      pointer.current = { x: event.clientX, y: event.clientY };
    };
    window.addEventListener("pointermove", move, { passive: true });
    return () => window.removeEventListener("pointermove", move);
  }, []);
  const { object, fitBox } = options;
  const fitSelection = useCallback(() => {
    const { selectedBodies, selected } = useWorkspace.getState();
    const meshes = bodyMeshes(
      object,
      selectedBodies.length ? selectedBodies : selected ? [selected] : [],
    );
    const box = new THREE.Box3();
    meshes.forEach((mesh) => box.expandByObject(mesh, true));
    fitBox(box.isEmpty() ? null : box);
  }, [object, fitBox]);
  const commands = viewportCommands({
    ...options,
    transformRef,
    pointer,
    fitSelection,
  });
  useViewportKeymap(commands, {
    enabled: options.enabled,
    emulateNumpad: options.emulateNumpad,
  });
  const { confirmEdit } = options;
  const onConfirm = useCallback(
    async (session: TransformSession, sceneToken: string) => {
      const store = useTransformViewport.getState();
      const project = useWorkspace.getState().project;
      const intents = transformIntents(session);
      if (!project?.currentRevision || !intents) return false;
      // A zero change neither asks for permission nor writes a revision.
      if (intents.every((item) => intentIsNoop(item.intent))) return false;
      const revisionId = project.currentRevision;
      const run = async () => {
        store.setStatus({ kind: "committing" });
        try {
          const result = await commitTransform(
            project,
            session,
            { projectId: project.id, revisionId, sceneToken },
            t("Transformed bodies in the viewport"),
          );
          store.setStatus({ kind: "idle" });
          if (result.status === "noop") return false;
          store.setLast(result.last);
          adopt(result.project);
          return true;
        } catch (cause) {
          store.setStatus({ kind: "error", message: errorText(cause) });
          return false;
        }
      };
      return confirmEdit
        ? confirmEdit(
            t(transformName[session.kind]),
            t("Apply the viewport transform as a new model revision."),
            t("{{value0}} bodies", { value0: session.targets.length }),
            run,
          )
        : run();
    },
    [adopt, confirmEdit],
  );
  return { transformRef, onConfirm, adopt };
}
