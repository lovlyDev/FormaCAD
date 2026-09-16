import { invoke, isTauri } from "@tauri-apps/api/core";
import { getLocale } from "../i18n";
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
): Promise<{ message: string; program: string | null }> {
  if (!native)
    throw new Error(
      "Local AI agents are available in the desktop app. You can edit model parameters here.",
    );
  return invoke("plan_model", {
    projectId: project.id,
    prompt,
    attachmentNames,
  });
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
export async function exportStep(projectId: string, parameters: Parameters) {
  return invoke<string>("export_step", { projectId, parameters });
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
  project: Project,
  program: string,
  prompt: string,
): Promise<Project> {
  return invoke("apply_program", {
    projectId: project.id,
    program,
    prompt,
    expectedRevision: project.currentRevision,
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
