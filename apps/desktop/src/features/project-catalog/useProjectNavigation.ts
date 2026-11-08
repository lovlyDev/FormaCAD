import {isSourceCurrent} from "../project-sources/sourceBinding";
import { useCallback, useEffect, useRef } from "react";
import { errorText, t } from "../../i18n";
import { useWorkspace } from "../../stores/workspace";
import type { Project } from "../../types";
import { readProjectSnapshot } from "./api";

// Route reads and explicit card selections share one generation. Neither a late
// response nor a failed catalog read may clear or overwrite a newer workspace.
export function useProjectNavigation(projectId: string | undefined, navigate: (path: string) => void, onError: (message: string) => void) {
  const generation = useRef(0);
  useEffect(() => () => { generation.current++; }, []);
  useEffect(() => {
    const request = ++generation.current;
    const before = useWorkspace.getState().project;
    const beforeSource=useWorkspace.getState().sourceBinding;
    if (!projectId) {
      if (before) useWorkspace.getState().setProject(null);
      return;
    }
    if (before?.id === projectId) return;
    const fail = (cause: unknown) => {
      if (generation.current !== request || (beforeSource?!isSourceCurrent(useWorkspace.getState().sourceBinding,beforeSource):!!useWorkspace.getState().sourceBinding)) return;
      onError(errorText(cause));
      const active = useWorkspace.getState().project;
      if (active && active.id !== projectId) navigate("/project/" + active.id);
    };
    void readProjectSnapshot(projectId).then(latest => {
      if (generation.current !== request || (beforeSource?!isSourceCurrent(useWorkspace.getState().sourceBinding,beforeSource):!!useWorkspace.getState().sourceBinding)) return;
      if (useWorkspace.getState().project !== before || useWorkspace.getState().busy) {
        fail(new Error(t("The project changed while opening. Retry opening it."))); return;
      }
      useWorkspace.getState().openManagedProject(latest);
    }).catch(fail);
    return () => { generation.current++; };
  }, [projectId, navigate, onError]);

  return useCallback(async (selected: Project) => {
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
}
