import { invoke } from "@tauri-apps/api/core";
import { t } from "../../../i18n";
import type { Project } from "../../../types";
import { captureApply } from "../../model-apply/capturedApply";

export function captureTorus(
  project: Project,
  expectedRevision: string | null,
  majorRadiusMm: number,
  minorRadiusMm: number,
) {
  const name = t("Torus");
  const featureId = crypto.randomUUID();
  const bodyId = crypto.randomUUID();
  const commands = [
    {
      command: "add_feature",
      feature: {
        id: featureId,
        name,
        operation: {
          type: "torus",
          majorRadius: { kind: "literal", mm: majorRadiusMm },
          minorRadius: { kind: "literal", mm: minorRadiusMm },
        },
      },
    },
    { command: "add_body", body: { id: bodyId, name, sourceFeatureId: featureId } },
  ];
  const captured = captureApply(project, "", t("Add torus"));
  return {
    captured,
    invoke: () => invoke<Project>("apply_ir_commands", {
      projectId: captured.base.id,
      expectedRevision,
      prompt: captured.prompt,
      commands,
    }),
  };
}
