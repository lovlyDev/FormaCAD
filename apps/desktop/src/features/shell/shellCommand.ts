import { invoke } from "@tauri-apps/api/core";
import { Mesh, type Group } from "three";
import { t } from "../../i18n";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import type { Project } from "../../types";
import { captureApply } from "../model-apply/capturedApply";
import { faceReference, faceTriangleCounts, type FaceSelection } from "../viewer/faceSelection";
import { matchesReferenceRoute } from "../viewer/topology/referenceRoute";
import type { FaceTopologyReference } from "../viewer/topology/topologyReference";

function shellTarget(project: Project, selection: FaceSelection) {
  if (!selection.topologyRef || selection.revisionId !== project.currentRevision) return null;
  const revision = project.revisions.find((item) => item.id === project.currentRevision);
  const document = readTypedCadDocument(revision?.program ?? "");
  const body = document?.bodies.find((item) => item.id === selection.bodyId);
  if (!document || !body || !matchesReferenceRoute(document, body.sourceFeatureId, selection.topologyRef))
    return null;
  return { document, body, primary: selection.topologyRef };
}

function sameOccurrence(a: FaceTopologyReference, b: FaceTopologyReference) {
  return a.ownerFeatureId === b.ownerFeatureId &&
    a.occurrencePath.join("/") === b.occurrencePath.join("/");
}

export function shellFaces(project: Project, selection: FaceSelection | null, object: Group) {
  if (!selection || selection.sceneToken !== object.uuid) return [];
  const target = shellTarget(project, selection);
  const mesh = object.getObjectByName(selection.bodyId);
  if (!target || !(mesh instanceof Mesh)) return [];
  const counts = faceTriangleCounts(mesh);
  if (!counts) return [];
  return counts.flatMap((_count, index) => {
    const reference = faceReference(mesh, index + 1);
    return reference && sameOccurrence(reference, target.primary) ? [reference] : [];
  });
}

export function captureShell(project: Project, selection: FaceSelection,
  openings: FaceTopologyReference[], offsetMm: number) {
  const target = shellTarget(project, selection);
  if (!target || !openings.length || openings.length > 16 ||
    !Number.isFinite(offsetMm) || Math.abs(offsetMm) <= 1e-7 || Math.abs(offsetMm) > 10000)
    return null;
  const { document, body, primary } = target;
  if (new Set(openings.map((face) => face.role)).size !== openings.length ||
      openings.some((face) => !sameOccurrence(primary, face) ||
        !matchesReferenceRoute(document, body.sourceFeatureId, face))) return null;
  const name = t("Shell");
  const feature = {
    id: crypto.randomUUID(), name,
    operation: { type: "shell", bodyFeatureId: body.sourceFeatureId, openings,
      offset: { kind: "literal", mm: offsetMm } },
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
        { command: "set_body_source", bodyId: body.id, sourceFeatureId: feature.id },
      ],
    }),
  };
}
