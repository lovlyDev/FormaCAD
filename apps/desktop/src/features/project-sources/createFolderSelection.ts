import { invoke, isTauri } from "@tauri-apps/api/core";
import { t, errorText } from "../../i18n";
import type { Project } from "../../types";
import {
  adoptFolderSelection, type FolderSelection, type FolderSelectionTarget,
} from "./openFolderSelection";

export type BlankFolderDraft = Pick<Project, "name" | "units" | "agent">;

type FolderCreationReply =
  | { status: "opened"; source: FolderSelection }
  | { status: "savedWithoutOpening" | "indeterminate"; commitId: string; error: string };

export function createFolderSelection(
  { name, units, agent }: BlankFolderDraft,
  target: FolderSelectionTarget,
  report: (message: string) => void,
) {
  return requestFolderCreation(target, report, () =>
    invoke<FolderCreationReply | null>("create_project_folder", { draft: { name, units, agent } }),
  );
}

export async function requestFolderCreation(
  target: FolderSelectionTarget,
  report: (message: string) => void,
  request: () => Promise<FolderCreationReply | null>,
): Promise<boolean> {
  if (!target.isCurrent()) return false;
  if (!isTauri()) throw new Error("PROJECT_FOLDER_UNSUPPORTED");
  let reply: FolderCreationReply | null;
  try {
    reply = await request();
  } catch (cause) {
    if (target.isCurrent()) throw cause;
    return false;
  }
  if (reply === null) return false;
  if (reply.status === "opened") return adoptFolderSelection(reply.source, target);
  if (!target.isCurrent()) return false;
  if ((reply.status !== "savedWithoutOpening" && reply.status !== "indeterminate") ||
      typeof reply.commitId !== "string" || !reply.commitId || typeof reply.error !== "string") {
    throw new Error("PROJECT_SNAPSHOT_CHANGED");
  }
  const message = reply.status === "savedWithoutOpening"
    ? t("The project was saved. Open the selected folder to continue.")
    : t("The save result is uncertain. Open the selected folder to check it.");
  report(`${message}\n${reply.commitId}\n${errorText(reply.error)}`);
  return true;
}
