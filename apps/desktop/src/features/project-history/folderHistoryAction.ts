import { useWorkspace } from "../../stores/workspace";
import { t } from "../../i18n";
import type { ProjectSourceBinding } from "../project-sources/sourceBinding";
import type { FolderWriteAction } from "../project-sources/useFolderWriteAction";
import type { HistoryDirection } from "./historyApi";
import type { useFolderWriteConfirmation } from "../project-sources/useFolderWriteConfirmation";

type Confirm = ReturnType<typeof useFolderWriteConfirmation>["confirm"];

export function folderHistoryAction(
  binding: ProjectSourceBinding | null,
  write: FolderWriteAction["write"],
  confirm: Confirm,
) {
  return async (direction: HistoryDirection) => {
    const project = useWorkspace.getState().project;
    if (!project || binding?.kind !== "folder" || project.id !== binding.projectId) return;
    const detail = `${project.name}\n${project.currentRevision ?? ""}`;
    await write(binding, {
      kind: "history",
      direction,
      expectedRevision: project.currentRevision,
    }, detail, () => confirm(binding, {
      title: t(direction === "undo" ? "Undo model change" : "Redo model change"),
      description: t("This changes the current model as one history step. Saved revisions stay available."),
      detail,
    }));
  };
}
