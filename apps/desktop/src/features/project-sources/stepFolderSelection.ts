import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import { requestFolderCreation, type BlankFolderDraft } from "./createFolderSelection";
import type { FolderSelectionTarget } from "./openFolderSelection";

export type StepFolderDraft = Pick<BlankFolderDraft, "name" | "agent">;

export function transferStepToFolder(
  { name, agent }: StepFolderDraft,
  target: FolderSelectionTarget,
  report: (message: string) => void,
) {
  return requestFolderCreation(target, report, () => invoke("transfer_step_to_folder", {
    draft: { name, agent, prompt: t("Imported geometry from the selected STEP") },
  }));
}
