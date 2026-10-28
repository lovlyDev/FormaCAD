import { useEffect } from "react";
import { X, LayoutDashboard } from "lucide-react";
import { t } from "../../i18n";
import { usePersistentState } from "../../lib/persistence";
import type { Project } from "../../types";
import "./projectTabs.css";
const empty: string[] = [];
export function ProjectTabs({
  projects,
  activeId,
  blocked,
  onSelect,
  onHome,
}: {
  projects: Project[];
  activeId: string;
  blocked: boolean;
  onSelect: (project: Project) => void;
  onHome: () => void;
}) {
  const [stored, setStored] = usePersistentState<string[]>(
    "forma.ui.projectTabs",
    empty,
  );
  const ids = Array.isArray(stored)
    ? stored.filter(
        (id, index) =>
          typeof id === "string" &&
          stored.indexOf(id) === index &&
          projects.some((project) => project.id === id),
      )
    : [];
  useEffect(() => {
    setStored((current) => {
      const value = Array.isArray(current)
        ? current.filter((id) => typeof id === "string")
        : [];
      return value.includes(activeId) ? value : [...value, activeId].slice(-30);
    });
  }, [activeId, setStored]);
  return (
    <div
      className="project-tabs"
      role="tablist"
      aria-label={t("Open projects")}
    >
      <button aria-label={t("Dashboard")} disabled={blocked} onClick={onHome}>
        <LayoutDashboard size={14} />
      </button>
      {ids.map((id) => {
        const project = projects.find((item) => item.id === id)!;
        return (
          <div
            key={id}
            className={`project-tab ${id === activeId ? "is-active" : ""}`}
          >
            <button
              role="tab"
              aria-selected={id === activeId}
              disabled={blocked}
              onClick={() => onSelect(project)}
            >
              {project.name}
            </button>
            <button
              aria-label={t("Close project tab {{value0}}", {
                value0: project.name,
              })}
              disabled={blocked}
              onClick={() => {
                const next = ids.filter((value) => value !== id);
                setStored(next);
                if (id === activeId) {
                  const target = projects.find(
                    (item) => item.id === next.at(-1),
                  );
                  if (target) onSelect(target);
                  else onHome();
                }
              }}
            >
              <X size={12} />
            </button>
          </div>
        );
      })}
    </div>
  );
}
