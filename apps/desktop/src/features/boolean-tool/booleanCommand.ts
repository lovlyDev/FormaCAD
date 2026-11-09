import { invoke } from "@tauri-apps/api/core";
import type { Project } from "../../types";
import { t } from "../../i18n";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import { captureApply } from "../model-apply/capturedApply";

export const booleanModes = ["union", "cut", "intersect"] as const;
export type BooleanMode = typeof booleanModes[number];
export const booleanLabels = {
  union: "Boolean union",
  cut: "Boolean cut",
  intersect: "Boolean intersection",
} as const;

function savedDocument(project: Project) {
  const revision = project.revisions.find((item) => item.id === project.currentRevision);
  const document = readTypedCadDocument(revision?.program ?? "");
  return document?.revisionId === project.currentRevision ? document : null;
}

export function booleanBodies(project: Project) {
  return savedDocument(project)?.bodies ?? [];
}

export function captureBoolean(project: Project, targetId: string, toolId: string, mode: BooleanMode) {
  const document = savedDocument(project);
  const target = document?.bodies.find((body) => body.id === targetId);
  const tool = document?.bodies.find((body) => body.id === toolId);
  if (!document || !target || !tool || target === tool) return null;
  const name = t(booleanLabels[mode]);
  const feature = {
    id: crypto.randomUUID(), name,
    operation: { type: "boolean", leftFeatureId: target.sourceFeatureId, rightFeatureId: tool.sourceFeatureId, mode },
  };
  const bodies = document.bodies.map((body) => body === target ? { ...body, sourceFeatureId: feature.id } : body);
  const captured = captureApply(project, "", name);
  return {
    captured,
    previewSource: JSON.stringify({ ...document, features: [...document.features, feature], bodies }),
    invoke: () => invoke<Project>("apply_ir_commands", {
      projectId: captured.base.id,
      expectedRevision: captured.base.currentRevision,
      prompt: captured.prompt,
      commands: [
        { command: "add_feature", feature },
        { command: "set_body_source", bodyId: target.id, sourceFeatureId: feature.id },
      ],
    }),
  };
}
