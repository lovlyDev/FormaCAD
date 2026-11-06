import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import type { Project } from "../../types";
import { captureApply } from "../model-apply/capturedApply";

export function duplicateBodyTarget(project: Project, bodyId: string) {
  const revision = project.revisions.find((item) => item.id === project.currentRevision);
  const document = readTypedCadDocument(revision?.program ?? "");
  return document?.revisionId === project.currentRevision
    ? document.bodies.find((item) => item.id === bodyId)
    : undefined;
}

export function captureDuplicateBody(project: Project, bodyId: string, name: string,
  offsetMm: [number, number, number]) {
  if (!duplicateBodyTarget(project, bodyId) || !name.trim() ||
      offsetMm.some((value) => !Number.isFinite(value) || Math.abs(value) > 10000))
    return null;
  const newBodyId = crypto.randomUUID();
  const captured = captureApply(project, "", t("Duplicate body"));
  return {
    captured,
    invoke: () => invoke<Project>("apply_ir_commands", {
      projectId: captured.base.id,
      expectedRevision: captured.base.currentRevision,
      prompt: captured.prompt,
      commands: [{ command: "duplicate_body", bodyId, newBodyId, name: name.trim(), offsetMm }],
    }),
  };
}
