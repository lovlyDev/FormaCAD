import { invoke, isTauri } from "@tauri-apps/api/core";
import { z } from "zod";
import type { FolderSourceRef } from "./contract";

const id = z.string().uuid();
const requestSchema = z.object({
  projectId: id,
  sourceId: z.string().min(1).max(128),
  commitId: id,
});
const replySchema = requestSchema.extend({
  status: z.enum(["current", "historical", "absent"]),
});

export async function observeFolderCommit(
  projectId: string,
  source: FolderSourceRef,
  attemptedCommitId: string,
): Promise<z.infer<typeof replySchema>> {
  if (!isTauri()) throw new Error("PROJECT_FOLDER_UNSUPPORTED");
  const request = requestSchema.safeParse({
    projectId,
    sourceId: source.sourceId,
    commitId: attemptedCommitId,
  });
  if (!request.success) throw new Error("PROJECT_SNAPSHOT_CHANGED");
  const reply = replySchema.safeParse(
    await invoke<unknown>("observe_folder_commit", request.data),
  );
  if (
    !reply.success
    || reply.data.projectId !== request.data.projectId
    || reply.data.sourceId !== request.data.sourceId
    || reply.data.commitId !== request.data.commitId
  ) {
    throw new Error("PROJECT_SNAPSHOT_CHANGED");
  }
  return reply.data;
}
