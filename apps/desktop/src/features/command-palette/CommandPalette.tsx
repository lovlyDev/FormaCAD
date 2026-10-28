import { Command, Folder, Search } from "lucide-react";
import { t } from "../../i18n";
import { Modal } from "../../components/ui";
import type { Project } from "../../types";
import { useState } from "react";
import { projectSearchCommands } from "./projectSearchCommands";
import { settingsPages } from "../settings/settingsPages";
import "./commandPalette.css";

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
  extraCommands?: { label: string; run: () => void }[];
  activeProject?: Project | null;
  onSettingsPage?: (page: string) => void;
  onEntity?: (
    kind: "body" | "feature" | "parameter" | "revision",
    id: string,
  ) => void;
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
  extraCommands = [],
  activeProject,
  onSettingsPage = onSettings,
  onEntity,
}: Props) {
  const [active, setActive] = useState(0);
  const query = search.toLowerCase();
  const commands: {
    label: string;
    shortcut: string;
    run: () => void;
    project?: boolean;
  }[] = [
    { label: t("New project"), shortcut: "Ctrl N", run: onNew },
    { label: t("Import model"), shortcut: "Ctrl O", run: onImport },
    { label: t("Export model"), shortcut: "Ctrl Shift E", run: onExport },
    { label: t("Edit model parameters"), shortcut: "", run: onParameters },
    { label: t("Ask AI"), shortcut: "Ctrl Enter", run: onAsk },
    { label: t("Open settings"), shortcut: "", run: onSettings },
    ...extraCommands.map((command) => ({ ...command, shortcut: "" })),
    ...projectSearchCommands(activeProject ?? null, onEntity ?? (() => {})).map(
      (command) => ({ ...command, shortcut: "" }),
    ),
    ...settingsPages.map((page) => ({
      label: `${t("Settings")}: ${t(page)}`,
      shortcut: "",
      run: () => onSettingsPage(page),
    })),
    ...projects
      .filter((project) =>
        [
          project.name,
          ...project.files.map((file) => file.name),
          ...project.messages.map((message) => message.text),
          ...project.revisions.map((revision) => revision.prompt),
        ].some((value) => value.toLowerCase().includes(query)),
      )
      .map((project) => ({
        label: project.name,
        shortcut: "",
        run: () => onProject(project),
        project: true,
      })),
  ];
  const matching = commands
    .filter(
      (command) =>
        command.project || command.label.toLowerCase().includes(query),
    )
    .slice(0, 100);
  const activeIndex = Math.max(0, Math.min(active, matching.length - 1));
  return (
    <Modal
      open={open}
      onClose={close}
      title={t("Command palette")}
      description={t("Find a project or jump to an action.")}
    >
      <div
        className="palette-search"
        onKeyDown={(event) => {
          if (event.key === "ArrowDown" || event.key === "ArrowUp") {
            event.preventDefault();
            setActive((current) =>
              matching.length
                ? (current +
                    (event.key === "ArrowDown" ? 1 : -1) +
                    matching.length) %
                  matching.length
                : 0,
            );
          }
          if (event.key === "Enter" && matching[activeIndex]) {
            event.preventDefault();
            close();
            matching[activeIndex].run();
          }
        }}
      >
        <Search size={17} />
        <input
          autoFocus
          aria-label={t("Search commands and projects…")}
          placeholder={t("Search commands and projects…")}
          value={search}
          onChange={(event) => {
            onSearch(event.target.value);
            setActive(0);
          }}
        />
        <kbd>ESC</kbd>
      </div>
      <div className="palette-list">
        {!matching.length && (
          <p role="status">{t("No matching commands or projects")}</p>
        )}
        {matching.slice(0, 100).map((command, index) => (
          <button
            className={activeIndex === index ? "active" : ""}
            key={`${command.label}:${index}`}
            onClick={() => {
              close();
              command.run();
            }}
          >
            {command.project ? <Folder size={15} /> : <Command size={15} />}
            {command.label}
            <kbd>{command.shortcut}</kbd>
          </button>
        ))}
      </div>
    </Modal>
  );
}
