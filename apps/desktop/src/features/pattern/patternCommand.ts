import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import type { Project } from "../../types";
import { captureApply } from "../model-apply/capturedApply";

export type PatternVector = [number, number, number];
export type PatternLayout =
  | { type: "patternLinear"; layout: { countU: number; stepUMm: PatternVector;
    countV: number; stepVMm: PatternVector } }
  | { type: "patternCircular"; layout: { count: number; axisOriginMm: PatternVector;
    axisDirection: PatternVector; stepDeg: number } };

export function patternTarget(project: Project, bodyId: string) {
  const revision = project.revisions.find((item) => item.id === project.currentRevision);
  const document = readTypedCadDocument(revision?.program ?? "");
  const body = document?.bodies.find((item) => item.id === bodyId);
  const source = document?.features.find((item) => item.id === body?.sourceFeatureId);
  return body && source && document?.revisionId === project.currentRevision
    ? { body, canRepeatHole: source.operation.type === "hole" && !source.suppressed }
    : null;
}

export function capturePattern(project: Project, bodyId: string, repeatHole: boolean,
  pattern: PatternLayout) {
  const target = patternTarget(project, bodyId);
  if (!target || (repeatHole && !target.canRepeatHole)) return null;
  const feature = {
    id: crypto.randomUUID(), name: t(pattern.type),
    operation: { ...pattern, bodyFeatureId: target.body.sourceFeatureId, repeatHole },
  };
  const captured = captureApply(project, "", feature.name);
  return {
    captured,
    invoke: () => invoke<Project>("apply_ir_commands", {
      projectId: captured.base.id,
      expectedRevision: captured.base.currentRevision,
      prompt: captured.prompt,
      commands: [
        { command: "add_feature", feature },
        { command: "set_body_source", bodyId, sourceFeatureId: feature.id },
      ],
    }),
  };
}
