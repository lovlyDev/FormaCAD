import { openFolderSelection, type FolderSelectionTarget } from "../project-sources/openFolderSelection";
import { createFolderSelection, type BlankFolderDraft } from "../project-sources/createFolderSelection";
import { transferStepToFolder, type StepFolderDraft } from "../project-sources/stepFolderSelection";
import { recoverFolderSelection } from "../project-sources/recoverFolderSelection";
import { isSourceCurrent, type ProjectSourceBinding } from "../project-sources/sourceBinding";
import { useCallback, useEffect, useRef } from "react";
import { errorText, t } from "../../i18n";
import { useWorkspace } from "../../stores/workspace";
import type { Project } from "../../types";
import { readProjectSnapshot } from "./api";

// Route reads and explicit card selections share one generation. Neither a late
// response nor a failed catalog read may clear or overwrite a newer workspace.
export function useProjectNavigation(
  projectId: string | undefined,
  navigate: (path: string) => void,
  onError: (message: string) => void,
  folderRoute = false,
) {
  const generation = useRef(0);
  const pickerActive = useRef(false);
  useEffect(() => () => { generation.current++; }, []);
  useEffect(() => {
    const request = ++generation.current;
    const before = useWorkspace.getState().project;
    const beforeSource = useWorkspace.getState().sourceBinding;
    if (folderRoute) {
      if (before?.id && before.id === projectId && beforeSource?.kind === "folder" && beforeSource.projectId === projectId) return;
      // Only an explicit selection may restore a folder after a renderer reload.
      navigate("/");
      return;
    }
    if (!projectId) {
      if (before) useWorkspace.getState().setProject(null);
      return;
    }
    if (before?.id === projectId && beforeSource?.kind !== "folder") return;
    const fail = (cause: unknown) => {
      if (generation.current !== request || (beforeSource?!isSourceCurrent(useWorkspace.getState().sourceBinding,beforeSource):!!useWorkspace.getState().sourceBinding)) return;
      onError(errorText(cause));
      const active = useWorkspace.getState().project;
      if (active && beforeSource?.kind === "folder") navigate("/folder/" + active.id);
      else if (active && active.id !== projectId) navigate("/project/" + active.id);
    };
    void readProjectSnapshot(projectId).then(latest => {
      if (generation.current !== request || (beforeSource?!isSourceCurrent(useWorkspace.getState().sourceBinding,beforeSource):!!useWorkspace.getState().sourceBinding)) return;
      if (useWorkspace.getState().project !== before || useWorkspace.getState().busy) {
        fail(new Error(t("The project changed while opening. Retry opening it."))); return;
      }
      useWorkspace.getState().openManagedProject(latest);
    }).catch(fail);
    return () => { generation.current++; };
  }, [projectId, navigate, onError, folderRoute]);

  const openProject = useCallback(async (selected: Project) => {
    if (useWorkspace.getState().busy) return;
    const request = ++generation.current;
    const before = useWorkspace.getState().project;
    const beforeSource=useWorkspace.getState().sourceBinding;
    try {
      const latest = await readProjectSnapshot(selected.id);
      if (generation.current !== request || (beforeSource?!isSourceCurrent(useWorkspace.getState().sourceBinding,beforeSource):!!useWorkspace.getState().sourceBinding)) return;
      if (useWorkspace.getState().project !== before || useWorkspace.getState().busy) {
        onError(t("The project changed while opening. Retry opening it.")); return;
      }
      useWorkspace.getState().openManagedProject(latest);
      navigate("/project/" + latest.id);
    } catch (cause) { if (generation.current === request && (beforeSource?isSourceCurrent(useWorkspace.getState().sourceBinding,beforeSource):!useWorkspace.getState().sourceBinding)) onError(errorText(cause)); }
  }, [navigate, onError]);

  const selectFolder = useCallback(async <Result,>(select: (target: FolderSelectionTarget) => Promise<Result>) => {
    const before = useWorkspace.getState();
    if (before.busy || pickerActive.current) return;
    const request = ++generation.current;
    const isCurrent = () => {
      const active = useWorkspace.getState();
      return request === generation.current && !active.busy &&
        active.project === before.project && sameOpening(active.sourceBinding, before.sourceBinding);
    };

    pickerActive.current = true;
    try {
      return await select({
        isCurrent,
        activate(project, source) {
          useWorkspace.getState().openFolderProject(project, source);
          navigate("/folder/" + project.id);
        },
      });
    } catch (cause) {
      if (isCurrent()) onError(errorText(cause));
    } finally {
      pickerActive.current = false;
    }
  }, [navigate, onError]);

  return {
    openProject,
    openFolder: (recentLocation?: string) => selectFolder(target => openFolderSelection(target, recentLocation)),
    createFolder: (draft: BlankFolderDraft) =>
      selectFolder(target => createFolderSelection(draft, target, onError)),
    transferStep: (draft: StepFolderDraft) =>
      selectFolder(target => transferStepToFolder(draft, target, onError)),
    recoverFolder: () => selectFolder(target => recoverFolderSelection(target, onError)),
  };
}

function sameOpening(active: ProjectSourceBinding | null, captured: ProjectSourceBinding | null) {
  return captured ? isSourceCurrent(active, captured) : !active;
}
