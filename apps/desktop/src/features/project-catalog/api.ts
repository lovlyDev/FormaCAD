import { invoke } from "@tauri-apps/api/core";
import { t } from "../../i18n";
import { listProjects, native } from "../../lib/api";
import type { Project } from "../../types";
import type { CatalogReport } from "./contract";

export async function listProjectCatalog(): Promise<CatalogReport> {
  if (native) return invoke("list_projects");
  const projects = await listProjects();
  return { entries: projects.map(project => ({ kind: "ready" as const, rowKey: project.id, project })), serializedProjectBytes: new TextEncoder().encode(JSON.stringify(projects)).length };
}
export async function readProjectSnapshot(id: string): Promise<Project> {
  if (native) return invoke("read_project_snapshot", { id });
  const project = (await listProjects()).find(item => item.id === id);
  if (!project) throw new Error(t("Project was not found"));
  return project;
}
