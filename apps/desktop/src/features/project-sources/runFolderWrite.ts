import { requestNativePermission, resolveNativePermission } from "../../lib/api";
import { handleFolderWriteResult, isFolderWriteOpeningCurrent as isCurrent } from "./folderWriteResult";
import type { FolderPermissionTicket, FolderWriteIntent } from "./permissionContract";
import type { ProjectSourceBinding } from "./sourceBinding";
import { executeFolderWrite, type FolderWriteCommand } from "./writeApi";

type Opening = Extract<ProjectSourceBinding, { kind: "folder" }>;

function commandFor(intent: FolderWriteIntent): FolderWriteCommand {
  switch (intent.kind) {
    case "model": return "program";
    case "commands": return "commands";
    case "history": return intent.direction;
  }
}

export async function runFolderWrite(
  captured: Opening,
  intent: FolderWriteIntent,
  detail: string,
  confirm: () => Promise<boolean>,
) {
  const stale = { status: "stale" } as const;
  if (!isCurrent(captured)) return stale;
  const command = commandFor(intent);
  const ticket = await requestNativePermission(captured.projectId, "modify_project", detail, {
    source: captured.source,
    intent,
  });
  if (ticket.requiresConfirmation) {
    if (!isCurrent(captured)) {
      await discard(ticket);
      return stale;
    }
    let allow: boolean;
    try {
      allow = await confirm();
    } catch (error) {
      await discard(ticket);
      throw error;
    }
    if (!isCurrent(captured)) {
      await discard(ticket);
      return stale;
    }
    await resolveNativePermission(ticket.id, allow, ticket);
    if (!allow) return isCurrent(captured) ? { status: "denied" } as const : stale;
  }
  if (!isCurrent(captured)) return stale;
  return handleFolderWriteResult(captured, await executeFolderWrite(ticket, command));
}

async function discard(ticket: Readonly<FolderPermissionTicket>) {
  // A close may already have revoked the ticket. Never mask the original outcome.
  await resolveNativePermission(ticket.id, false, ticket).catch(() => {});
}
