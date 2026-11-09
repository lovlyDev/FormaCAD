import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import type { Project } from "../../types";
import { captureApply } from "../model-apply/capturedApply";
import type { FaceSelection } from "../viewer/faceSelection";
import { matchesReferenceRoute } from "../viewer/topology/referenceRoute";

export function faceExtrusionTarget(project: Project, selection: FaceSelection | null) {
  if (!selection?.topologyRef || selection.revisionId !== project.currentRevision) return null;
  const revision = project.revisions.find((item) => item.id === project.currentRevision);
  const document = readTypedCadDocument(revision?.program ?? "");
  const body = document?.bodies.find((item) => item.id === selection.bodyId);
  const reference = selection.topologyRef;
  if (!document || !body || !matchesReferenceRoute(document, body.sourceFeatureId, reference))
    return null;
  if (!reference.role.startsWith("box-face:") &&
      reference.role !== "cylinder-face:top" && reference.role !== "cylinder-face:bottom")
    return null;
  return { bodyId: body.id, bodyFeatureId: body.sourceFeatureId, reference };
}

export function captureFaceExtrusion(project: Project, selection: FaceSelection, distanceMm: number) {
  const target = faceExtrusionTarget(project, selection);
  if (!target || !Number.isFinite(distanceMm) || Math.abs(distanceMm) <= 1e-7 ||
      Math.abs(distanceMm) > 10000) return null;
  const { bodyId, bodyFeatureId, reference } = target;
  const name = t("Extrude face");
  const feature = {
    id: crypto.randomUUID(),
    name,
    operation: {
      type: "extrudeFace",
      bodyFeatureId,
      reference,
      distance: { kind: "literal", mm: distanceMm },
    },
  };
  const captured = captureApply(project, "", name);
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
