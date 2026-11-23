import { invoke } from "@tauri-apps/api/core";
import { errorText, getLocale, t } from "../../i18n";
import { isFolderWriteOpeningCurrent } from "../project-sources/folderWriteResult";
import type { FolderWriteOpening } from "../project-sources/useFolderWriteAction";

export async function exportFolderBundle(
  opening: FolderWriteOpening,
  redactConversation: boolean,
  reportSaved: (text: string) => void,
  reportError: (text: string) => void,
) {
  if (!isFolderWriteOpeningCurrent(opening)) return;
  try {
    const saved = await invoke<string | null>("export_folder_project_bundle", {
      projectId: opening.projectId,
      sourceId: opening.source.sourceId,
      expectedCommitId: opening.source.commitId,
      locale: getLocale(),
      redactConversation,
    });
    if (saved && isFolderWriteOpeningCurrent(opening)) reportSaved(t("Project bundle exported"));
  } catch (cause) {
    if (isFolderWriteOpeningCurrent(opening)) reportError(errorText(cause));
  }
}
