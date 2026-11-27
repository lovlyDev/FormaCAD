import { invoke, isTauri } from "@tauri-apps/api/core";
import type { FolderSourceRef } from "./contract";

/** Close the window-owned selector; never accept or resolve a renderer path. */
export async function closeFolderSource(source: FolderSourceRef): Promise<boolean> {
  if (!isTauri()) return false;
  return invoke<boolean>("close_project_source", { sourceId: source.sourceId });
}
