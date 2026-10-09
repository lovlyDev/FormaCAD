import { ArrowUpRight, Command, Folder, Search } from "lucide-react";
import { t } from "../../i18n";
import { Modal } from "../../components/ui";
import type { Project } from "../../types";

interface Props {
  open: boolean;
  close: () => void;
  search: string;
  onSearch: (value: string) => void;
  projects: Project[];
  onProject: (project: Project) => void;
  onNew: () => void;
  onImport: () => void;
  onExport: () => void;
  onParameters: () => void;
  onAsk: () => void;
  onSettings: () => void;
}

export function CommandPalette({
  open,
  close,
  search,
  onSearch,
  projects,
  onProject,
  onNew,
  onImport,
  onExport,
  onParameters,
  onAsk,
  onSettings,
}: Props) {
  const query = search.toLowerCase();
  const commands = [
    { label: t("New project"), shortcut: "Ctrl N", run: onNew },
    { label: t("Import model"), shortcut: "Ctrl O", run: onImport },
    { label: t("Export model"), shortcut: "Ctrl Shift E", run: onExport },
    { label: t("Edit model parameters"), shortcut: "", run: onParameters },
    { label: t("Ask AI"), shortcut: "Ctrl Enter", run: onAsk },
    { label: t("Open settings"), shortcut: "", run: onSettings },
  ];
  return (
    <Modal
      open={open}
      onClose={close}
      title={t("Command palette")}
      description={t("Find a project or jump to an action.")}
    >
      <div className="palette-search">
        <Search size={17} />
        <input
          autoFocus
          placeholder={t("Search commands and projects…")}
          value={search}
          onChange={(event) => onSearch(event.target.value)}
        />
        <kbd>ESC</kbd>
      </div>
      <div className="palette-list">
        {commands
          .filter((command) => command.label.toLowerCase().includes(query))
          .map((command) => (
            <button key={command.label} onClick={command.run}>
              <Command size={15} />
              {command.label}
              <kbd>{command.shortcut}</kbd>
            </button>
          ))}
        {projects
          .filter((project) =>
            [
              project.name,
              ...project.files.map((file) => file.name),
              ...project.messages.map((message) => message.text),
              ...project.revisions.map((revision) => revision.prompt),
            ].some((value) => value.toLowerCase().includes(query)),
          )
          .map((project) => (
            <button key={project.id} onClick={() => onProject(project)}>
              <Folder size={15} />
              {project.name}
              <ArrowUpRight size={13} />
            </button>
          ))}
      </div>
    </Modal>
  );
}
