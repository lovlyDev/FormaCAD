import { useWorkspace } from "../../stores/workspace";
import { t, errorText } from "../../i18n";
import { candidateSourceStatus } from "../agents/candidateSource";
import { moveModelHistory, type HistoryDirection } from "./historyApi";

type Confirm = (title: string, description: string, detail: string, run: () => Promise<void>) => Promise<void>;
export function useHistoryAction(confirm: Confirm, refresh: () => void) {
  return (direction: HistoryDirection) => {
    const base = useWorkspace.getState().project;
    if (!base || useWorkspace.getState().busy) return;
    void confirm(t(direction === "undo" ? "Undo model change" : "Redo model change"),
      t("This changes the current model as one history step. Saved revisions stay available."),
      `${base.name}\n${base.currentRevision ?? ""}`, async () => {
        const workspace = useWorkspace.getState();
        if (candidateSourceStatus(workspace.project, base) !== "valid") throw new Error(t("Model changed before the history action. Try again."));
        workspace.setBusy(true, t(direction === "undo" ? "Undoing model change…" : "Redoing model change…"));
        try {
          const result = await moveModelHistory(base, direction);
          if (candidateSourceStatus(useWorkspace.getState().project, base) !== "valid") return;
          useWorkspace.getState().setProject(result);
        } catch (error) { useWorkspace.getState().setError(errorText(error)); }
        finally { useWorkspace.getState().setBusy(false); refresh(); }
      });
  };
}
