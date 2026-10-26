import { invoke } from "@tauri-apps/api/core";
import { getLocale } from "../../i18n";
import type { Project } from "../../types";

export type FieldType = "length" | "angle" | "number" | "integer" | "boolean" | "choice" | "text";
export type FieldValue = number | boolean | string;

export interface FormField {
  key: string;
  type: FieldType;
  label: string;
  help?: string;
  default?: FieldValue;
  min?: number;
  max?: number;
  options?: { value: string; label: string }[];
}

export interface Contribution {
  id: string;
  title: string;
  icon?: string;
  when?: "always" | "model" | "selection";
  params: FormField[];
}

export interface FormatContribution {
  id: string;
  title: string;
  extensions: string[];
  params: FormField[];
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  about?: string;
  categories: string[];
  author: { name: string; github: string };
  repository: string;
  license: string;
  icon?: string;
  iconColor?: string;
  minAppVersion: string;
  api: string;
  permissions: string[];
  contributes: {
    features: Contribution[];
    commands: Contribution[];
    importers: FormatContribution[];
    exporters: FormatContribution[];
    panels: { id: string; title: string; icon?: string; location: "sidebar" | "inspector" }[];
    settings: FormField[];
  };
  links?: { homepage?: string; funding?: string; issues?: string };
  locales: string[];
}

export type Translations = Record<string, Record<string, string>>;

export interface InstallPreview {
  token: string;
  manifest: PluginManifest;
  permissions: string[];
  addedPermissions: string[];
  installedVersion: string | null;
  isUpdate: boolean;
  isDowngrade: boolean;
  alreadyInstalled: boolean;
  source: { kind: "file" } | { kind: "catalog"; pageUrl: string };
  sha256: string;
  translations: Translations;
}

export interface PluginSummary {
  manifest: PluginManifest;
  versions: { version: string; sha256: string; source: string; installedAt: string }[];
  active: string;
  enabled: boolean;
  acceptedPermissions: string[];
  settings: Record<string, FieldValue>;
  translations: Translations;
  problem?: string;
}

export interface Notice {
  level: "info" | "warning" | "error";
  message: string;
}

export interface RunOutcome {
  project: Project | null;
  notices: Notice[];
  message: string | null;
}

export interface JournalEntry {
  at: string;
  pluginId: string;
  version?: string;
  event: string;
  ok: boolean;
  detail: string;
  durationMs?: number;
}

export type PanelBlock =
  | { type: "text"; text: string }
  | { type: "form"; fields: FormField[]; submit: { command: string; label: string } };

export const listPlugins = () => invoke<PluginSummary[]>("plugins_list");
export const installFromFile = (title: string, filterName: string) =>
  invoke<InstallPreview | null>("plugins_install_file", { title, filterName });
export const installFromLink = (url: string) => invoke<InstallPreview>("plugins_install_link", { url });
export const takePendingLink = () => invoke<string | null>("plugins_take_pending_link");
export const confirmInstall = (token: string) => invoke<PluginSummary>("plugins_install_confirm", { token });
export const cancelInstall = (token: string) => invoke<void>("plugins_install_cancel", { token });
export const setPluginEnabled = (id: string, enabled: boolean) =>
  invoke<PluginSummary>("plugins_set_enabled", { id, enabled });
export const setActiveVersion = (id: string, version: string, accepted: string[]) =>
  invoke<PluginSummary>("plugins_set_active", { id, version, accepted });
export const savePluginSettings = (id: string, values: Record<string, FieldValue>) =>
  invoke<PluginSummary>("plugins_set_settings", { id, values });
export const uninstallPlugin = (id: string, version?: string) =>
  invoke<void>("plugins_uninstall", { id, version: version ?? null });
export const pluginJournal = (id?: string) => invoke<JournalEntry[]>("plugins_journal", { id: id ?? null });

export function runPlugin(request: {
  project: Project;
  pluginId: string;
  kind: "feature" | "command";
  contributionId: string;
  params: Record<string, FieldValue>;
  prompt: string;
  selection?: unknown;
}) {
  return invoke<RunOutcome>("plugins_run", {
    request: {
      projectId: request.project.id,
      pluginId: request.pluginId,
      kind: request.kind,
      contributionId: request.contributionId,
      params: request.params,
      expectedRevision: request.project.currentRevision ?? null,
      prompt: request.prompt,
      locale: getLocale(),
      selection: request.selection ?? null,
    },
  });
}

export function importWithPlugin(request: {
  project: Project;
  pluginId: string;
  importerId: string;
  params: Record<string, FieldValue>;
  prompt: string;
  title: string;
}) {
  return invoke<RunOutcome | null>("plugins_import", {
    request: {
      projectId: request.project.id,
      pluginId: request.pluginId,
      importerId: request.importerId,
      params: request.params,
      expectedRevision: request.project.currentRevision ?? null,
      prompt: request.prompt,
      locale: getLocale(),
      title: request.title,
    },
  });
}

export function exportWithPlugin(request: {
  project: Project;
  pluginId: string;
  exporterId: string;
  params: Record<string, FieldValue>;
  fileName: string;
}) {
  return invoke<{ path: string | null; notices: Notice[] }>("plugins_export", {
    projectId: request.project.id,
    pluginId: request.pluginId,
    exporterId: request.exporterId,
    params: request.params,
    fileName: request.fileName,
    locale: getLocale(),
  });
}

export const renderPanel = (projectId: string | null, pluginId: string, panelId: string) =>
  invoke<{ view: { blocks: PanelBlock[] }; notices: Notice[] }>("plugins_panel", {
    projectId,
    pluginId,
    panelId,
    locale: getLocale(),
  });
