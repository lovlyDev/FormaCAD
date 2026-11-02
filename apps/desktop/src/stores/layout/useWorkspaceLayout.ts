import { useCallback, useMemo } from "react";
import { usePersistentState } from "../../lib/persistence";
import {
  defaultLayout,
  parseLayout,
  type WorkspaceLayout,
  type WorkspacePreset,
} from "../../features/workspace-areas/layoutModel";
export interface SavedWorkspace {
  id: string;
  name: string;
  layout: WorkspaceLayout;
}
function initialWorkspaces(): SavedWorkspace[] {
  return (["model", "sketch", "inspect"] as WorkspacePreset[]).map((id) => ({
    id,
    name: "",
    layout: defaultLayout(id),
  }));
}
const initial = initialWorkspaces();
export function useWorkspaceLayout(projectId: string) {
  const prefix = `forma.ui.project.${projectId}.areas.v1`;
  const [stored, setStored] = usePersistentState<unknown>(
    `${prefix}.workspaces`,
    initial,
  );
  const [activeId, setActiveId] = usePersistentState(
    `${prefix}.active`,
    "model",
  );
  const workspaces = useMemo<SavedWorkspace[]>(() => {
    if (!Array.isArray(stored) || !stored.length || stored.length > 20)
      return initial;
    const ids = new Set<string>();
    return stored.flatMap((item) => {
      if (
        !item ||
        typeof item.id !== "string" ||
        !/^[a-zA-Z0-9_-]{1,100}$/.test(item.id) ||
        ids.has(item.id) ||
        typeof item.name !== "string"
      )
        return [];
      ids.add(item.id);
      return [
        {
          id: item.id,
          name: item.name.slice(0, 80),
          layout: parseLayout(item.layout),
        },
      ];
    });
  }, [stored]);
  const safeWorkspaces = workspaces.length ? workspaces : initial;
  const active =
    safeWorkspaces.find((item) => item.id === activeId) ?? safeWorkspaces[0];
  const change = useCallback(
    (layout: WorkspaceLayout) =>
      setStored(
        safeWorkspaces.map((item) =>
          item.id === active.id ? { ...item, layout } : item,
        ),
      ),
    [setStored, safeWorkspaces, active.id],
  );
  return {
    active,
    workspaces: safeWorkspaces,
    change,
    activate: setActiveId,
    create: (name: string) => {
      if (!name.trim() || safeWorkspaces.length >= 20) return;
      const id = crypto.randomUUID();
      setStored([
        ...safeWorkspaces,
        { id, name: name.trim().slice(0, 80), layout: active.layout },
      ]);
      setActiveId(id);
    },
    rename: (name: string) => {
      if (name.trim())
        setStored(
          safeWorkspaces.map((item) =>
            item.id === active.id
              ? { ...item, name: name.trim().slice(0, 80) }
              : item,
          ),
        );
    },
    reset: () =>
      change(
        defaultLayout(
          (["model", "inspect", "sketch"].includes(active.id)
            ? active.id
            : "model") as WorkspacePreset,
        ),
      ),
  };
}
