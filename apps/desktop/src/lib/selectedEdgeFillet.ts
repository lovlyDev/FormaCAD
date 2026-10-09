import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import type { EdgeSelection } from "../features/viewer/edgeSelection";
import type { Project } from "../types";
import { readTypedCadDocument } from "./typedCadDocument";
import { edgeFilletDraft } from "../features/model-editor/edgeFilletDraft";

export async function filletSelectedEdge(project: Project, selection: EdgeSelection, radiusMm: number): Promise<Project> {
  const revision = project.revisions.find((item) => item.id === project.currentRevision);
  const document = revision?.program ? readTypedCadDocument(revision.program) : null;
  const body = document?.bodies.find((item) => item.id === selection.bodyId);
  if (!project.currentRevision || selection.revisionId !== project.currentRevision || !body)
    throw new Error(t("Selected edge is no longer available."));
  if (!Number.isFinite(radiusMm) || radiusMm <= 0 || radiusMm > 10000)
    throw new Error(t("Enter a valid fillet radius."));
  const draft = edgeFilletDraft(revision!.program!, selection, project.currentRevision, radiusMm);
  const next = draft ? readTypedCadDocument(draft) : null;
  const feature = next?.features.at(-1);
  if (!feature) throw new Error(t("Selected edge is no longer available."));
  return invoke("apply_ir_commands", {
    projectId: project.id,
    expectedRevision: project.currentRevision,
    prompt: t("Filleted selected edge"),
    commands: [
      { command: "add_feature", feature },
      { command: "set_body_source", bodyId: body.id, sourceFeatureId: feature.id },
    ],
  });
}
