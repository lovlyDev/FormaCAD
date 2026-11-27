import { invoke, isTauri } from "@tauri-apps/api/core";
import { errorText, t } from "../../i18n";
import { isFolderWriteOpeningCurrent } from "./folderWriteResult";
import type { FolderWriteOpening } from "./useFolderWriteAction";

type CopyReply =
  | { status: "saved"; commitId: string }
  | { status: "indeterminate"; commitId: string; error: string };

export async function saveFolderCopy(
  opening: FolderWriteOpening,
  reportSaved: (message: string) => void,
  reportError: (message: string) => void,
) {
  if (!isFolderWriteOpeningCurrent(opening)) return;
  try {
    if (!isTauri()) throw new Error("PROJECT_FOLDER_UNSUPPORTED");
    const reply = await invoke<CopyReply | null>("save_project_folder_copy", {
      projectId: opening.projectId,
      sourceId: opening.source.sourceId,
      expectedCommitId: opening.source.commitId,
    });
    if (!isFolderWriteOpeningCurrent(opening) || reply === null) return;
    if (typeof reply.commitId !== "string" || !reply.commitId) throw new Error("PROJECT_SNAPSHOT_CHANGED");
    if (reply.status === "saved") {
      reportSaved(t("Project copy saved"));
    } else if (reply.status === "indeterminate" && typeof reply.error === "string") {
      reportError(`${t("The copy result is uncertain. Open the destination folder to check it.")}\n${reply.commitId}\n${errorText(reply.error)}`);
    } else {
      throw new Error("PROJECT_SNAPSHOT_CHANGED");
    }
  } catch (cause) {
    if (isFolderWriteOpeningCurrent(opening)) reportError(errorText(cause));
  }
}
