import {isSourceCurrent} from "../project-sources/sourceBinding";
import { readProjectSnapshot } from "../project-catalog/api";
import { invalidateProjectCatalog } from "../project-catalog/cache";
import { useQueryClient } from "@tanstack/react-query";
import { useWorkspace } from "../../stores/workspace";
import { applyProgram, saveProject } from "../../lib/api";
import { t, errorText } from "../../i18n";
import type { Project } from "../../types";
import type { ReviewPlan } from "../agents/review/plan/reviewPlan";
import { executeCapturedApply, type CapturedApply } from "./capturedApply";

/** One adopted geometry result, with independently reported optional conversation persistence. */
export function useModelApply() {
  const query = useQueryClient();
  return async (captured: CapturedApply, options?: { reviewPlan?: ReviewPlan | null; assistantMessage?: string }) => {
    const origin=useWorkspace.getState().sourceBinding;
    const sameOrigin=()=>origin?isSourceCurrent(useWorkspace.getState().sourceBinding,origin):!useWorkspace.getState().sourceBinding;
    const cache = (project: Project) => query.setQueryData<Project[]>(["projects"], current => {
      const items = current ?? [];
      return items.some(item => item.id === project.id) ? items.map(item => item.id === project.id ? project : item) : [...items, project];
    });
    const adopt = (project: Project) => { cache(project); if(sameOrigin())useWorkspace.getState().setProject(project); };
    const targetError = (message: string) => {
      if (sameOrigin()&&useWorkspace.getState().project?.id === captured.base.id&&(captured.sourceOpeningId===undefined||captured.sourceOpeningId===useWorkspace.getState().sourceBinding?.openingId)) useWorkspace.getState().setError(message);
    };
    try {
      if(origin?.kind==="folder")throw new Error("PROJECT_SOURCE_WRITE_NOT_AUTHORIZED");
      const outcome = await executeCapturedApply(captured, {
        getCurrent: () => sameOrigin()?useWorkspace.getState().project:null,
        getSourceOpeningId:()=>useWorkspace.getState().sourceBinding?.openingId,
        invoke: request => applyProgram(request.base, request.program, request.prompt, options?.reviewPlan),
        adopt,
        reconcileTarget: async result => {
          const before = useWorkspace.getState().project;
          const latest = await readProjectSnapshot(result.id);
          cache(latest);
          const current = useWorkspace.getState().project;
          if (sameOrigin()&&current?.id === latest.id && current.currentRevision === before?.currentRevision && current.id === before?.id) adopt(latest);
        },
        saveCommittedMetadata: options?.assistantMessage === undefined ? undefined : async result => {
          if(useWorkspace.getState().project?.id===result.id&&!sameOrigin())throw new Error("PROJECT_SNAPSHOT_CHANGED");
          const latest = await readProjectSnapshot(result.id);
          if(useWorkspace.getState().project?.id===result.id&&!sameOrigin())throw new Error("PROJECT_SNAPSHOT_CHANGED");
          const saved = await saveProject({ ...latest, messages: [...latest.messages, {
            id: crypto.randomUUID(), role: "assistant", text: options.assistantMessage!, createdAt: new Date().toISOString(),
          }] });
          cache(saved);
          const current = useWorkspace.getState().project;
          if (sameOrigin()&&current?.id === saved.id && current.currentRevision === saved.currentRevision) adopt(saved);
        },
        metadataFailed: (_result, cause) => targetError(`${t("The model was saved, but the assistant message could not be saved.")}\n${errorText(cause)}`),
        refreshTarget: () => {
          invalidateProjectCatalog(query);
          void query.invalidateQueries({ queryKey: ["model-history", captured.base.id] });
        },
      });
      if (outcome.adoptionFailure) targetError(`${t("The model was saved, but the workspace could not be refreshed.")}\n${errorText(outcome.adoptionFailure)}`);
      return outcome;
    } catch (cause) {
      targetError(errorText(cause));
      throw cause;
    }
  };
}
