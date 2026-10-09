import { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { Undo2, Redo2 } from "lucide-react";
import { IconButton } from "../../components/ui";
import { native } from "../../lib/api";
import { t, errorText } from "../../i18n";
import type { Project } from "../../types";
import { modelHistoryStatus, type HistoryDirection } from "./historyApi";
import { historyShortcut } from "./historyShortcut";
import "./ModelHistoryControls.css";

export function ModelHistoryControls({ project, blocked, onAction }: { project: Project; blocked: boolean; onAction: (direction: HistoryDirection) => void }) {
  const status = useQuery({ queryKey: ["model-history", project.id, project.currentRevision, project.revisions.length], queryFn: () => modelHistoryStatus(project.id), enabled: native, retry: false });
  const fresh = status.data?.expectedRevision === project.currentRevision && !status.isFetching;
  const undo = native && fresh && !!status.data?.canUndo && !blocked;
  const redo = native && fresh && !!status.data?.canRedo && !blocked;
  useEffect(() => {
    const handle = (event: KeyboardEvent) => {
      const direction = historyShortcut(event);
      if (!direction || (direction === "undo" ? !undo : !redo)) return;
      event.preventDefault(); onAction(direction);
    };
    window.addEventListener("keydown", handle);
    return () => window.removeEventListener("keydown", handle);
  }, [undo, redo, onAction]);
  return <div className="model-history-controls" role="group" aria-label={t("Model undo and redo")}>
    <IconButton label={t("Undo model (Ctrl+Z)")} disabled={!undo} onClick={() => onAction("undo")}><Undo2 size={16} /></IconButton>
    <IconButton label={t("Redo model (Ctrl+Y)")} disabled={!redo} onClick={() => onAction("redo")}><Redo2 size={16} /></IconButton>
    {status.error && <span className="model-history-error" role="status" title={errorText(status.error)}>{t("Model history is unavailable")}</span>}
  </div>;
}
