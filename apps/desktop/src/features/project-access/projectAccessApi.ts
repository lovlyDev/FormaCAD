import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type { Project } from "../../types";
const status = z.object({ projectId: z.string(), mode: z.enum(["write", "read_only", "closed"]), ownerPid: z.number().int().nonnegative().nullable() });
export type ProjectAccess = z.infer<typeof status>;
export async function acquireProjectAccess(projectId: string): Promise<ProjectAccess> {
  return status.parse(await invoke("acquire_project_access", { projectId }));
}
export async function releaseProjectAccess(projectId: string): Promise<void> { await invoke("release_project_access", { projectId }); }
export async function copyProjectForEditing(projectId: string): Promise<Project> { return invoke("copy_project_for_editing", { projectId }); }
