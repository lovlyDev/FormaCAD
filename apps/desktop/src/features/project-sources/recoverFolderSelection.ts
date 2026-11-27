import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import { requestFolderCreation, type FolderCreationReply } from "./createFolderSelection";
import type { FolderSelectionTarget } from "./openFolderSelection";

export function recoverFolderSelection(
  target: FolderSelectionTarget,
  report: (message: string) => void,
) {
  return requestFolderCreation(target, report, () =>
    invoke<FolderCreationReply | null>("recover_project_folder_copy", {
      titles: {
        source: t("Select the folder to recover"),
        destination: t("Select an empty folder for the recovered copy"),
      },
    }),
  );
}
