import { invoke, isTauri } from "@tauri-apps/api/core";
import { z } from "zod";
import type { Project } from "../../types";
import {
  capturePermissionContext,
  type FolderPermissionTicket,
} from "./permissionContract";

const commands = {
  program: "apply_program",
  commands: "apply_ir_commands",
  undo: "undo_model",
  redo: "redo_model",
} as const;

export type FolderWriteCommand = keyof typeof commands;

const id = z.string().uuid();
const sourceKey = z.string().min(1).max(128);
const envelope = { sourceId: sourceKey, projectId: id, baseCommitId: id };
// The host validates persisted Project data. Check the outer shape and identity
// here without parsing or copying the full history and attachment payload.
const list = z.custom<unknown[]>(Array.isArray);
const projectShape = z.object({
  schemaVersion: z.literal(1),
  id,
  name: z.string(),
  units: z.enum(["mm", "cm", "inch"]),
  agent: z.enum(["codex", "claude", "custom"]),
  pinned: z.boolean(),
  createdAt: z.string(),
  updatedAt: z.string(),
  currentRevision: z.string().nullable(),
  revisions: list,
  messages: list,
  files: list,
  exports: list,
});
const project = z.custom<Project>((value) => projectShape.safeParse(value).success);
const reply = z.discriminatedUnion("status", [
  z.strictObject({ ...envelope, status: z.literal("noOp"), project }),
  z.strictObject({ ...envelope, status: z.literal("committed"), commitId: id, project }),
  z.strictObject({ ...envelope, status: z.literal("rejectedBefore"), error: z.string() }),
  z.strictObject({ ...envelope, status: z.literal("indeterminate"), commitId: id, error: z.string() }),
]);

export type FolderWriteResult = z.infer<typeof reply>;

export async function executeFolderWrite(
  ticket: FolderPermissionTicket,
  command: FolderWriteCommand,
): Promise<FolderWriteResult> {
  if (!isTauri()) throw new Error("PROJECT_FOLDER_UNSUPPORTED");
  const { projectId, source } = capturePermissionContext(ticket);
  const approvalId = ticket.id;
  if (
    !sourceKey.safeParse(source.sourceId).success
    || ![projectId, source.commitId, approvalId].every((value) => id.safeParse(value).success)
  ) {
    throw new Error("PROJECT_SNAPSHOT_CHANGED");
  }
  const result = reply.safeParse(await invoke<unknown>(commands[command], {
    projectId,
    folderWrite: {
      sourceId: source.sourceId,
      expectedCommitId: source.commitId,
      approvalId,
    },
  }));
  if (!result.success) throw new Error("PROJECT_SNAPSHOT_CHANGED");
  const value = result.data;
  if (
    value.projectId !== projectId
    || value.sourceId !== source.sourceId
    || value.baseCommitId !== source.commitId
    || ("project" in value && value.project.id !== projectId)
    || ("commitId" in value && value.commitId === source.commitId)
  ) {
    throw new Error("PROJECT_SNAPSHOT_CHANGED");
  }
  return value;
}
