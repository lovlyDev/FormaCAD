import { invoke } from "@tauri-apps/api/core";
import { native } from "./api";
import type { TypedCadDocument } from "./typedCadDocument";

type Snapshot<T> = { index: number; value: T };
type EntityChange<T> = { id: string; before: Snapshot<T> | null; after: Snapshot<T> | null };
export interface CadDocumentDiff {
  fromRevisionId: string;
  toRevisionId: string;
  parameters: EntityChange<TypedCadDocument["parameters"][number]>[];
  features: EntityChange<TypedCadDocument["features"][number]>[];
  bodies: EntityChange<TypedCadDocument["bodies"][number]>[];
}

export async function compareCadRevisions(projectId: string, fromRevisionId: string, toRevisionId: string): Promise<CadDocumentDiff | null> {
  if (!native) return null;
  return invoke("compare_ir_revisions", { projectId, fromRevisionId, toRevisionId });
}
