import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type { Project } from "../../types";
import { getLocale } from "../../i18n";

const inspection = z.object({ path: z.string(), projectId: z.string(), name: z.string(), currentRevision: z.string().nullable(), revisionCount: z.number().int().nonnegative(), manifestSha256: z.string().regex(/^[a-fA-F0-9]{64}$/), identityExists: z.boolean() });
export type ProjectFolderInspection = z.infer<typeof inspection>;
export async function inspectProjectFolder(): Promise<ProjectFolderInspection | null> {
  const value = await invoke("inspect_project_folder", { locale: getLocale() });
  return value === null ? null : inspection.parse(value);
}
export async function importProjectFolder(folder: ProjectFolderInspection, saveCopy: boolean): Promise<Project> {
  return invoke("import_project_folder", { path: folder.path, expectedManifestSha256: folder.manifestSha256, saveCopy });
}
