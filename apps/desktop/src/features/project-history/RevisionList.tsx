import { GitBranch, Undo2 } from "lucide-react";
import { t, getLocale } from "../../i18n";
import type { Project, Revision } from "../../types";
import { revisionHistoryLabel } from "./revisionHistoryLabel";
export function RevisionList({
  project,
  blocked,
  compare,
  onRestore,
  onCompare,
}: {
  project: Project;
  blocked: boolean;
  compare: string | null;
  onRestore: (revision: Revision) => void;
  onCompare: (id: string | null) => void;
}) {
  return (
    <div className="revision-list">
      {[...project.revisions].reverse().map((revision, index) => (
        <div
          id={`revision-${revision.id}`}
          className={`revision-item ${revision.id === project.currentRevision ? "current" : ""}`}
          key={revision.id}
        >
          <div>
            <GitBranch size={14} />
            <strong>
              {t("Revision")} {project.revisions.length - index}
            </strong>
            {revision.id === project.currentRevision && (
              <span>{t("Current")}</span>
            )}
          </div>
          <p>
            {revisionHistoryLabel(revision.prompt) ??
              (revision.prompt.startsWith(
                "Добавь анимацию движения к текущей сборке.",
              )
                ? t("Настроено движение сборки")
                : revision.prompt)}
          </p>
          <small>
            {new Date(revision.createdAt).toLocaleString(getLocale())}
          </small>
          <div className="revision-actions">
            <button
              disabled={blocked || revision.id === project.currentRevision}
              onClick={() => onRestore(revision)}
            >
              <Undo2 size={12} />
              {t("Restore")}
            </button>
            <button
              disabled={blocked}
              onClick={() =>
                onCompare(compare === revision.id ? null : revision.id)
              }
            >
              {compare === revision.id ? t("Close comparison") : t("Compare")}
            </button>
          </div>
        </div>
      ))}
      {!project.revisions.length && (
        <p className="field-hint">{t("No revisions")}</p>
      )}
    </div>
  );
}
