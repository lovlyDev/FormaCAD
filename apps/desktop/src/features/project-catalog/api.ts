import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import { listProjects, native } from "../../lib/api";
import type { Project } from "../../types";
import type { CatalogReport } from "./contract";
import type { FolderSourceRef } from "../project-sources/contract";

export async function listProjectCatalog(): Promise<CatalogReport> {
  if (native) return invoke("list_projects");
  const projects = await listProjects();
  return { entries: projects.map(project => ({ kind: "ready" as const, rowKey: project.id, project })), serializedProjectBytes: new TextEncoder().encode(JSON.stringify(projects)).length };
}
export async function readProjectSnapshot(id: string, source?: FolderSourceRef): Promise<Project> {
  if (native) return invoke("read_project_snapshot", source
    ? { id, sourceId: source.sourceId, expectedCommitId: source.commitId }
    : { id });
  if (source) throw new Error(t("PROJECT_FOLDER_UNSUPPORTED"));
  const project = (await listProjects()).find(item => item.id === id);
  if (!project) throw new Error(t("Project was not found"));
  return project;
}
