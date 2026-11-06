import type { Workspace } from "./workspace";
import type { Project } from "../types";
import { folderSource, isSourceCurrent, managedSource, refreshFolderSource, type ProjectSourceBinding } from "../features/project-sources/sourceBinding";
import { closeFolderSource } from "../features/project-sources/sourceApi";
import { readState } from "../lib/persistence";
import { saveProject } from "../lib/api";

type Actions = Pick<Workspace, "setProject" | "openManagedProject" | "openFolderProject" | "publishFolderSnapshot" | "update">;
export function workspaceProjectActions(set: (patch: Partial<Workspace>) => void, get: () => Workspace): Actions {
  function publish(project: Project | null, sourceBinding: ProjectSourceBinding | null) {
    const before = get();
    const sameSource = !!sourceBinding && !!before.sourceBinding && isSourceCurrent(sourceBinding, before.sourceBinding);
    const sameRevision = sameSource && !!project && project.currentRevision === before.project?.currentRevision;
    const selected = sameRevision ? before.selected : project && sourceBinding?.kind === "managed"
      ? readState<string | null>(`forma.ui.project.${project.id}.selected`, null) : null;
    set({ project, sourceBinding, selected,
      selectedBodies: sameSource ? before.selectedBodies : selected ? [selected] : [],
      selectedFace: sameRevision ? before.selectedFace : null,
      selectedEdge: sameRevision ? before.selectedEdge : null, error: null });
    const previous = before.sourceBinding;
    if (previous?.kind === "folder" && !(sourceBinding?.kind === "folder" && sourceBinding.source.sourceId === previous.source.sourceId)) {
      void closeFolderSource(previous.source).catch(cause => console.warn("Folder source could not be closed", cause));
    }
  }
  return {
    // Existing managed metadata publication preserves its renderer request family.
    setProject(project) {
      const before = get();
      const previous = before.project?.id === project?.id ? before.sourceBinding : null;
      publish(project, project ? managedSource(project.id, previous) : null);
    },
    openManagedProject(project) { publish(project, managedSource(project.id)); },
    openFolderProject(project, source) { publish(project, folderSource(project.id, source)); },
    publishFolderSnapshot(project, captured, source) {
      const before = get();
      if (!before.sourceBinding || !isSourceCurrent(before.sourceBinding, captured) ||
          captured.kind !== "folder" || before.sourceBinding.kind !== "folder" ||
          before.sourceBinding.source.commitId !== captured.source.commitId || project.id !== captured.projectId) return false;
      publish(project, refreshFolderSource(before.sourceBinding, source));
      return true;
    },
    async update(project) {
      const before = get(), captured = before.sourceBinding;
      if (captured?.kind === "folder") throw new Error("PROJECT_SOURCE_WRITE_NOT_AUTHORIZED");
      if (!captured || captured.projectId !== project.id || before.project?.id !== project.id) throw new Error("PROJECT_SNAPSHOT_CHANGED");
      const saved = await saveProject(project);
      const active = get();
      if (!isSourceCurrent(active.sourceBinding, captured) || saved.id !== captured.projectId) return;
      const sameRevision = saved.currentRevision === active.project?.currentRevision;
      set({ project: saved, selectedFace: sameRevision ? active.selectedFace : null, selectedEdge: sameRevision ? active.selectedEdge : null });
    },
  };
}
