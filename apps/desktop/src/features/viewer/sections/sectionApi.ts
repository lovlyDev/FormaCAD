import type { FolderSourceRef } from "../../project-sources/contract";
import { native } from "../../../lib/api";
import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type { SectionPlane } from "./sectionPlane";
const vector = z.tuple([z.number().finite(), z.number().finite(), z.number().finite()]);
const report = z.object({ sourceSha256: z.string().regex(/^[a-f0-9]{64}$/), geometry: z.object({
  plane: z.object({ originMm: vector, normal: vector, deflectionMm: z.number().positive() }),
  totalLengthMm: z.number().finite().nonnegative(),
  curves: z.array(z.object({ id: z.string(), lengthMm: z.number().finite().nonnegative(), closed: z.boolean(), pointsMm: z.array(vector).min(2).max(100000) })).max(4096)
    .refine(curves => curves.reduce((count, curve) => count + curve.pointsMm.length, 0) <= 100000),
}) });
export type SectionReport = z.infer<typeof report>;
export async function sectionModel(projectId: string, expectedRevision: string, sourceSha256: string, plane: SectionPlane, source?: FolderSourceRef): Promise<SectionReport> {
  if(source&&!native)throw Error("PROJECT_FOLDER_UNSUPPORTED");
  const result = report.parse(await invoke("section_model", { projectId, expectedRevision, ...plane, ...(source ? {sourceId:source.sourceId,expectedCommitId:source.commitId} : {}) }));
  if (result.sourceSha256 !== sourceSha256) throw new Error("SECTION_STALE");
  return result;
}
