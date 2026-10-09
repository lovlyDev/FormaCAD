import { version as appVersion } from "../../package.json";
import { useQueryClient } from "@tanstack/react-query";
import { motion } from "framer-motion";
import {
  Box, Cpu, FolderOpen, LayoutGrid, LoaderCircle, PackageOpen, Pencil, Plus, Search, Settings2, Star, Trash2,
} from "lucide-react";
import { Button, IconButton } from "../components/ui";
import { errorText, getLocale, quantity, t } from "../i18n";
import { native, saveProject } from "../lib/api";
import { currentParameters } from "../stores/workspace";
import type { Project } from "../types";
import { ImportProjectFolderButton } from "../features/project-folder/ImportProjectFolderButton";
import { ExportProjectBundleControl } from "../features/project-bundle/ExportProjectBundleControl";

type Props = {
  projects: Project[];
  loading: boolean;
  error: unknown;
  search: string;
  onSearch: (value: string) => void;
  onSettings: () => void;
  onImport: () => void;
  onImportBundle: () => void;
  onExportBundle: (project: Project, redactConversation: boolean) => void;
  onNew: () => void;
  onOpen: (project: Project) => void;
  onRename: (project: Project) => void;
  onDelete: (project: Project) => void;
  onRefresh: () => void;
  onError: (message: string) => void;
};

export function Dashboard({ projects, loading, error, search, onSearch, onSettings, onImport, onImportBundle, onExportBundle, onNew, onOpen, onRename, onDelete, onRefresh, onError }: Props) {
  const queryClient = useQueryClient();
  const visible = [...projects]
    .sort((a, b) => Number(b.pinned) - Number(a.pinned))
    .filter((project) => [
      project.name,
      ...project.files.map((file) => file.name),
      ...project.messages.map((message) => message.text),
      ...project.revisions.map((revision) => revision.prompt),
    ].some((text) => text.toLowerCase().includes(search.toLowerCase())));

  function togglePin(project: Project) {
    const current = queryClient.getQueryData<Project[]>(["projects"]) ?? [];
    const latest = current.find((item) => item.id === project.id) ?? project;
    const next = { ...latest, pinned: !latest.pinned };
    queryClient.setQueryData<Project[]>(["projects"], current.map((item) => item.id === project.id ? next : item));
    void saveProject(next).then(onRefresh).catch((cause) => {
      queryClient.setQueryData(["projects"], current);
      onError(errorText(cause));
    });
  }

  return <main className="dashboard">
    <div className="dashboard-nav">
      <div><LayoutGrid size={17} />{t("All projects")}</div>
      <button onClick={onSettings}><Cpu size={17} />{t("AI agents")}</button>
      <button onClick={onSettings}><Settings2 size={17} />{t("Settings")}</button>
    </div>
    <div className="dashboard-content">
      <div className="eyebrow">{t("YOUR IDEAS, IN DIMENSIONS")}</div>
      <div className="dashboard-title">
        <div><h1>{t("A place to make things.")}</h1><p>{t("Give your next idea a little more shape.")}</p></div>
        <div className="dashboard-import-actions"><Button onClick={onImport}><FolderOpen size={15} />{t("Import model")}</Button>{native && <><Button onClick={onImportBundle}><PackageOpen size={15} />{t("Import project bundle")}</Button><ImportProjectFolderButton onImported={project => { onRefresh(); onOpen(project); }} onError={onError} /></>}</div>
      </div>
      <div className="section-heading">
        <h2>{t("Projects")}</h2>
        <div className="search-input"><Search size={14} /><input aria-label={t("Search projects")} placeholder={t("Find a project…")} value={search} onChange={(event) => onSearch(event.target.value)} /></div>
      </div>
      {loading ? <div className="empty-state"><LoaderCircle className="spin" />{t("Loading projects…")}</div>
        : error ? <div className="error-inline">{errorText(error)}</div>
          : <div className="project-grid">
            <button className="new-project-card" onClick={onNew}><span><Plus size={24} /></span><strong>{t("Create a project")}</strong><small>{t("From a thought to a tangible thing")}</small></button>
            {visible.map((project) => <motion.article layout="position" transition={{ layout: { duration: 0.42, ease: [0.22, 1, 0.36, 1] } }} className={`project-card${project.pinned ? " is-pinned" : ""}`} key={project.id}>
              <button className="project-art" onClick={() => onOpen(project)}>
                {project.thumbnail && project.thumbnailRevision === project.currentRevision
                  ? <img src={project.thumbnail} alt={t("Preview of {{value0}}", { value0: project.name })} />
                  : <Box size={72} strokeWidth={0.7} />}
                {project.pinned && <span className="project-pinned"><Star size={12} fill="currentColor" />{t("Pinned")}</span>}
                <span>{currentParameters(project).kind.toUpperCase()}</span>
              </button>
              <div className="project-card-info">
                <button onClick={() => onOpen(project)}><strong>{project.name}</strong><small>{new Date(project.updatedAt).toLocaleDateString(getLocale(), { month: "short", day: "numeric" })} <span>·</span>{" "}{quantity("revisions", project.revisions.length)}</small></button>
                <div className="project-card-actions">
                  <IconButton label={project.pinned ? t("Unpin project") : t("Pin project")} aria-pressed={project.pinned} active={project.pinned} onClick={() => togglePin(project)}><Star size={15} fill={project.pinned ? "currentColor" : "none"} /></IconButton>
                  {native && <ExportProjectBundleControl project={project} onExport={onExportBundle} />}
                  <IconButton label={t("Rename project")} onClick={() => onRename(project)}><Pencil size={14} /></IconButton>
                  <IconButton label={t("Delete project")} onClick={() => onDelete(project)}><Trash2 size={14} /></IconButton>
                </div>
              </div>
            </motion.article>)}
          </div>}
      <div className="dashboard-footer"><span>FORMA / {appVersion}</span></div>
    </div>
  </main>;
}
