import { invoke } from "@tauri-apps/api/core";
import { native } from "./api";
import { t } from "../i18n";
import type { FolderSourceRef } from "../features/project-sources/contract";

export interface ExactModelProperties {
  volumeMm3: number;
  areaMm2: number;
  faceCount: number;
  edgeCount: number;
  boundsMm: [number, number, number];
}

export async function inspectSavedModel(
  projectId: string,
  revisionId: string,
  source?: FolderSourceRef,
): Promise<ExactModelProperties | null> {
  if (source && !native) throw new Error(t("PROJECT_FOLDER_UNSUPPORTED"));
  if (!native) return null;
  return invoke("inspect_model", source
    ? { projectId, revisionId, sourceId: source.sourceId, expectedCommitId: source.commitId }
    : { projectId, revisionId });
}
