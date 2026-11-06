import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import type { Project } from "../../types";
import { captureApply } from "../model-apply/capturedApply";

export type BodyGroup = { id: string; name: string; bodyIds: string[] };
export type GroupCommand =
  | { command: "group_create"; group: BodyGroup }
  | { command: "group_add"; groupId: string; bodyIds: string[] }
  | { command: "group_remove"; groupId: string; bodyIds?: string[] };

export function captureBodyGroupCommand(project: Project, command: GroupCommand) {
  const captured = captureApply(project, "", t("Edit body groups"));
  return {
    captured,
    invoke: () => invoke<Project>("apply_ir_commands", {
      projectId: captured.base.id, expectedRevision: captured.base.currentRevision,
      prompt: captured.prompt, commands: [command],
    }),
  };
}
