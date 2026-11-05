import { t } from "../../../i18n";
import { readTypedCadDocument } from "../../../lib/typedCadDocument";
import type { Project } from "../../../types";
import type { EdgeSelection } from "../../viewer/edgeSelection";
import type { FaceSelection } from "../../viewer/faceSelection";
import type { DatumSourceOption } from "./DatumDialog";
import { datumKind, readDatumOperation } from "./datumModel";

export function datumSources(project: Project, face: FaceSelection | null, edge: EdgeSelection | null): DatumSourceOption[] {
  const revision = project.revisions.find((item) => item.id === project.currentRevision);
  const document = revision?.program && readTypedCadDocument(revision.program);
  if (!revision || !document) return [];
  const sources: DatumSourceOption[] = [];
  if (face?.revisionId === revision.id && face.topologyRef && face.topologyRef.role !== "cylinder-face:side") {
    sources.push({ label: t("Selected planar face"),
      source: { type: "topology", bodyId: face.bodyId, reference: face.topologyRef } });
  }
  if (edge?.revisionId === revision.id && edge.topologyRef?.role.startsWith("box-edge:")) {
    sources.push({ label: t("Selected straight edge"),
      source: { type: "topology", bodyId: edge.bodyId, reference: edge.topologyRef } });
  }
  for (const feature of document.features) {
    const operation = readDatumOperation(feature.operation);
    if (operation && !feature.suppressed && datumKind(operation) === "datumPlane") {
      sources.push({ label: feature.name, source: { type: "plane", featureId: feature.id } });
    }
  }
  return sources;
}
