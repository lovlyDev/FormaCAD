import { invoke } from "@tauri-apps/api/core";
import { native } from "./api";

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
): Promise<ExactModelProperties | null> {
  if (!native) return null;
  return invoke("inspect_model", { projectId, revisionId });
}
