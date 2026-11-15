import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import type { Project } from "../../types";
import { captureApply } from "../model-apply/capturedApply";
import { edgeBlendTarget } from "../edge-blend/edgeBlendCommand";
import type { EdgeSelection } from "../viewer/edgeSelection";

export function captureVariableFillet(project: Project, selection: readonly EdgeSelection[], start: number, end: number) {
  if (selection.length !== 1 || ![start, end].every(value => Number.isFinite(value) && value > 1e-7 && value <= 10000)) return null;
  const target = edgeBlendTarget(project, selection);
  if (!target) return null;
  const { document, body, reference } = target;
  const name = t("Variable radius fillet");
  const feature = { id: crypto.randomUUID(), name, operation: {
    type: "variableFillet", bodyFeatureId: body.sourceFeatureId, reference,
    radiusStart: { kind: "literal", mm: start }, radiusEnd: { kind: "literal", mm: end },
  } };
  const captured = captureApply(project, "", name);
  return {
    captured,
    previewSource: JSON.stringify({ ...document, features: [...document.features, feature],
      bodies: document.bodies.map(item => item.id === body.id ? { ...item, sourceFeatureId: feature.id } : item) }),
    invoke: () => invoke<Project>("apply_ir_commands", {
      projectId: captured.base.id, expectedRevision: captured.base.currentRevision, prompt: captured.prompt,
      commands: [{ command: "add_feature", feature }, { command: "set_body_source", bodyId: body.id, sourceFeatureId: feature.id }],
    }),
  };
}
