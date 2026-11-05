import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import type { Project } from "../../types";
import { captureApply } from "../model-apply/capturedApply";
import type { EdgeSelection } from "../viewer/edgeSelection";
import { matchesReferenceRoute } from "../viewer/topology/referenceRoute";

export type EdgeBlendMode = "fillet" | "chamfer";

export function edgeBlendTarget(project: Project, selection: EdgeSelection | null) {
  const reference = selection?.topologyRef;
  if (!selection || !reference?.role.startsWith("box-edge:") ||
      selection.revisionId !== project.currentRevision) return null;
  const revision = project.revisions.find((item) => item.id === project.currentRevision);
  const document = readTypedCadDocument(revision?.program ?? "");
  const body = document?.bodies.find((item) => item.id === selection.bodyId);
  if (!document || !body || !matchesReferenceRoute(document, body.sourceFeatureId, reference))
    return null;
  return { document, body, reference };
}

export function captureEdgeBlend(project: Project, selection: EdgeSelection, mode: EdgeBlendMode, mm: number) {
  const target = edgeBlendTarget(project, selection);
  if (!target || !Number.isFinite(mm) || mm <= 0 || mm > 10000) return null;
  const { document, body, reference } = target;
  const name = mode === "fillet" ? t("Edge fillet") : t("chamfer");
  const size = { kind: "literal", mm };
  const source = { bodyFeatureId: body.sourceFeatureId, reference };
  const operation = mode === "fillet"
    ? { type: "filletReferencedEdge", ...source, radius: size }
    : { type: "chamferReferencedEdge", ...source, distance: size };
  const feature = { id: crypto.randomUUID(), name, operation };
  const captured = captureApply(project, "", name);
  const previewSource = JSON.stringify({
    ...document,
    features: [...document.features, feature],
    bodies: document.bodies.map((item) =>
      item.id === body.id ? { ...item, sourceFeatureId: feature.id } : item),
  });
  return {
    captured,
    previewSource,
    invoke: () => invoke<Project>("apply_ir_commands", {
      projectId: captured.base.id,
      expectedRevision: captured.base.currentRevision,
      prompt: captured.prompt,
      commands: [
        { command: "add_feature", feature },
        { command: "set_body_source", bodyId: body.id, sourceFeatureId: feature.id },
      ],
    }),
  };
}
