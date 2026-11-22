import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type { Project } from "../../types";
import type { FolderSourceRef } from "../project-sources/contract";

const statusSchema = z.object({ canUndo: z.boolean(), canRedo: z.boolean(), expectedRevision: z.string().regex(/^[A-Za-z0-9_-]{1,80}$/).nullable() });
export type ModelHistoryStatus = z.infer<typeof statusSchema>;
export type HistoryDirection = "undo" | "redo";
export async function modelHistoryStatus(
  projectId: string,
  source?: FolderSourceRef,
): Promise<ModelHistoryStatus> {
  if (source !== undefined && (!source?.sourceId || !source.commitId)) {
    throw new Error("PROJECT_SNAPSHOT_CHANGED");
  }
  const args = source
    ? { projectId, sourceId: source.sourceId, expectedCommitId: source.commitId }
    : { projectId };
  return statusSchema.parse(await invoke("history_status", args));
}
export async function moveModelHistory(project: Project, direction: HistoryDirection): Promise<Project> {
  return invoke(direction === "undo" ? "undo_model" : "redo_model", { projectId: project.id, expectedRevision: project.currentRevision });
}
