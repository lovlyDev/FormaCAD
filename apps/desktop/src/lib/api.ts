import { invoke, isTauri } from "@tauri-apps/api/core";
import { getLocale, t } from "../i18n";
import { reviewPlanSchema, type ReviewPlan } from "../features/agents/review/plan/reviewPlan";
import type { Project, Health, Parameters } from "../types";
import { cadDocumentSchema, type CadDocument } from "./cadDocument";
export const native = isTauri();
export interface InterruptedSession {
  id: string;
  projectId: string;
  createdAt: string;
}
export async function interruptedSessions(): Promise<InterruptedSession[]> {
  return native ? invoke("interrupted_sessions") : [];
}
export async function acknowledgeRecovery(id: string): Promise<void> {
  if (native) await invoke("acknowledge_recovery", { id });
}
const key = "forma.projects.v1";
export async function listProjects(): Promise<Project[]> {
  if (native) return invoke("list_projects");
  const raw = localStorage.getItem(key);
  if (!raw) return [];
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) throw new Error();
    return parsed as Project[];
  } catch {
    throw new Error(
      "Project storage cannot be read. Export or repair browser storage before continuing.",
    );
  }
}
export async function saveProject(project: Project): Promise<Project> {
  if (native) return invoke("save_project", { project });
  const all = await listProjects();
  localStorage.setItem(
    key,
    JSON.stringify([project, ...all.filter((p) => p.id !== project.id)]),
  );
  return project;
}
export async function exportProjectBundle(
  projectId: string,
  redactConversation = false,
): Promise<string | null> {
  return invoke("export_project_bundle", { projectId, locale: getLocale(), redactConversation });
}
export async function importProjectBundle(): Promise<Project | null> {
  return invoke("import_project_bundle");
}

export async function saveProjectThumbnail(
  id: string,
  revisionId: string,
  thumbnail: string,
): Promise<Project> {
  if (native)
    return invoke("save_project_thumbnail", { id, revisionId, thumbnail });
  const project = (await listProjects()).find((item) => item.id === id);
  if (!project) throw new Error("Project was not found");
  if (project.currentRevision !== revisionId) return project;
  return saveProject({ ...project, thumbnail, thumbnailRevision: revisionId });
}

export async function deleteProject(id: string): Promise<void> {
  if (native) await invoke("delete_project", { id });
  else {
    const projects = await listProjects();
    if (!projects.some((project) => project.id === id))
      throw new Error("Project was not found");
    localStorage.setItem(
      key,
      JSON.stringify(projects.filter((project) => project.id !== id)),
    );
  }
  for (const setting of Object.keys(localStorage)) {
    if (setting.startsWith(`forma.ui.project.${id}.`))
      localStorage.removeItem(setting);
  }
}

export async function readProjectFile(
  projectId: string,
  name: string,
): Promise<string> {
  return invoke("read_project_file", { projectId, name });
}
export async function health(): Promise<Health[]> {
  if (native) return invoke("detect_environment");
  return [
    {
      name: "Desktop runtime",
      available: false,
      detail: "Open the Tauri desktop app to connect local CLI agents.",
    },
    {
      name: "3D workspace",
      available: true,
      detail: "WebGL · local browser storage",
    },
    {
      name: "CAD kernel",
      available: false,
      detail: "STEP conversion requires the desktop CAD environment.",
    },
  ];
}
export async function plan(
  project: Project,
  prompt: string,
  attachmentNames: string[] = [],
  selection:
    | import("../features/viewer/faceSelection").FaceSelection
    | import("../features/viewer/edgeSelection").EdgeSelection
    | null = null,
): Promise<{ message: string; program: string | null; reviewPlan?: import("../features/agents/review/plan/reviewPlan").ReviewPlan | null }> {
  if (!native)
    throw new Error(
      "Local AI agents are available in the desktop app. You can edit model parameters here.",
    );
  const result = await invoke<{ message: string; program: string | null; reviewPlan?: unknown }>("plan_model", {
    projectId: project.id,
    prompt,
    attachmentNames,
    selection,
  });
  let reviewPlan: ReviewPlan | null = null;
  if (result.reviewPlan !== undefined && result.reviewPlan !== null) {
    const parsed = reviewPlanSchema.safeParse(result.reviewPlan);
    if (!parsed.success) throw new Error(t("AI review plan is invalid."));
    reviewPlan = parsed.data;
  }
  return { ...result, reviewPlan };
}
export async function cancelTask(projectId: string) {
  if (native) await invoke("cancel_task", { projectId });
}
export async function requestNativePermission(
  projectId: string,
  action: string,
  detail: string,
): Promise<string> {
  return invoke("request_permission", { projectId, action, detail });
}
export async function resolveNativePermission(id: string, allow: boolean) {
  return invoke("resolve_permission", { id, allow });
}
export async function exportStep(
  projectId: string,
  parameters: Parameters,
  bodyId: string | null,
) {
  return invoke<string | null>("export_step", { projectId, parameters, bodyId, locale: getLocale() });
}

export async function saveMesh(
  projectId: string,
  name: string,
  blob: Blob,
): Promise<string | null> {
  return invoke("export_mesh", {
    projectId,
    name,
    bytes: Array.from(new Uint8Array(await blob.arrayBuffer())),
    locale: getLocale(),
  });
}

export async function convertStep(
  projectId: string,
  name: string,
): Promise<Project> {
  return invoke("convert_step", { projectId, name });
}

export interface AuditEntry {
  id: string;
  projectId: string;
  action: string;
  detail: string;
  decision: string;
  createdAt: string;
}
export async function permissionAudit(): Promise<AuditEntry[]> {
  return native ? invoke("permission_audit") : [];
}

export interface ConfirmationSettings {
  mode: "all" | "cli" | "none";
  overrides: Record<string, boolean>;
}
export async function confirmationSettings(): Promise<ConfirmationSettings> {
  if (native) return invoke("get_confirmation_settings");
  return JSON.parse(
    localStorage.getItem("forma.confirmations") ??
      '{"mode":"all","overrides":{}}',
  );
}
export async function saveConfirmationSettings(value: ConfirmationSettings) {
  if (native) await invoke("set_confirmation_settings", { value });
  else localStorage.setItem("forma.confirmations", JSON.stringify(value));
}
export function needsConfirmation(
  settings: ConfirmationSettings,
  action: string,
) {
  return (
    settings.overrides[action] ??
    (settings.mode === "all" ||
      (settings.mode === "cli" &&
        ["run_agent", "install_dependency"].includes(action)))
  );
}

export async function applyProgram(
  project: Pick<Project, "id" | "currentRevision">,
  program: string,
  prompt: string,
  reviewPlan?: import("../features/agents/review/plan/reviewPlan").ReviewPlan | null,
): Promise<Project> {
  return invoke("apply_program", {
    projectId: project.id,
    program,
    prompt,
    expectedRevision: project.currentRevision,
    reviewPlan: reviewPlan ?? null,
  });
}

export async function applyCadDocument(
  project: Project,
  document: CadDocument,
  prompt: string,
): Promise<Project> {
  return applyProgram(
    project,
    JSON.stringify(cadDocumentSchema.parse(document), null, 2),
    prompt,
  );
}

export async function chooseCadPython(): Promise<string | null> {
  return invoke("choose_cad_python", { locale: getLocale() });
}

export interface CustomAgentConfig {
  executable: string;
  args: string[];
}

export async function getCustomAgentConfig(): Promise<CustomAgentConfig | null> {
  return native ? invoke("get_custom_agent_config") : null;
}

export async function setCustomAgentConfig(
  config: CustomAgentConfig | null,
): Promise<void> {
  if (native) await invoke("set_custom_agent_config", { config });
}
