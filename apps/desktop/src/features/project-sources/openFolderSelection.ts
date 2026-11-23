import { invoke, isTauri } from "@tauri-apps/api/core";
import { rawError } from "../../i18n";
import type { Project } from "../../types";
import type { FolderSourceRef } from "./contract";
import { closeFolderSource } from "./sourceApi";

export interface FolderSelection extends FolderSourceRef {
  readonly project: Project;
}

export interface FolderSelectionTarget {
  isCurrent(): boolean;
  activate(project: Project, source: FolderSourceRef): void;
}

/** Adopt a native picker result only while the initiating navigation still owns it. */
export async function openFolderSelection(target: FolderSelectionTarget, recentLocation?: string): Promise<void> {
  if (!target.isCurrent()) return;
  if (!isTauri()) throw new Error("PROJECT_FOLDER_UNSUPPORTED");

  let selected: FolderSelection | null;
  try {
    selected = recentLocation === undefined
      ? await invoke<FolderSelection | null>("open_project_folder")
      : await invoke<FolderSelection | null>("open_project_folder", { recentLocation });
  } catch (cause) {
    if (!target.isCurrent()) return;
    if (rawError(cause) === "PROJECT_SNAPSHOT_RECOVERY_REQUIRED") {
      throw new Error("This folder needs recovery. Use Recover a folder copy to keep the original files.", { cause });
    }
    throw cause;
  }
  if (!selected) return;
  await adoptFolderSelection(selected, target);
}

export async function adoptFolderSelection(selected: FolderSelection, target: FolderSelectionTarget): Promise<boolean> {
  const { project, sourceId, commitId, taskScopeId } = selected;
  const source = { sourceId, commitId, taskScopeId };
  let adopted = false;
  try {
    if (!target.isCurrent()) return false;
    if (!project?.id || !sourceId || !commitId || !taskScopeId) {
      throw new Error("PROJECT_SNAPSHOT_CHANGED");
    }
    target.activate(project, source);
    adopted = true;
    return true;
  } finally {
    if (!adopted && sourceId) {
      try {
        await closeFolderSource(source);
      } catch (cause) {
        console.warn("Folder source could not be closed", cause);
      }
    }
  }
}
