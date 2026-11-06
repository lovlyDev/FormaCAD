import { t, getLocale, systemText, errorText } from "../../i18n";
import { useQuery } from "@tanstack/react-query";
import { native } from "../../lib/api";
import type { Project } from "../../types";
import { projectActivity } from "./activityApi";

const labels = { edit: "Model revision saved", undo: "Model edit undone", redo: "Model edit redone" };

export function ActivityPanel({
  project,
  errorsOnly = false,
}: {
  project: Project;
  errorsOnly?: boolean;
}) {
  const history = useQuery({
    queryKey: ["project-activity", project.id, project.currentRevision, project.revisions.length],
    queryFn: () => projectActivity(project.id), enabled: native && !errorsOnly, retry: false,
  });
  const messages = project.messages.filter((message) =>
    errorsOnly
      ? message.role === "error"
      : message.role === "event" || message.role === "error",
  ).map((message) => ({
    id: `message-${message.id}`,
    at: message.createdAt,
    label: message.role === "event" ? systemText(message.text) : errorText(message.text),
    detail: undefined as string | undefined,
  }));
  const revisions = errorsOnly ? [] : (history.data ?? []).map((event) => ({
    id: `history-${event.id}`,
    at: event.createdAt,
    label: t(labels[event.kind]),
    detail: project.revisions.find((revision) => revision.id === event.toRevision)?.prompt,
  }));
  const entries = [...revisions, ...messages].sort((a, b) => Date.parse(b.at) - Date.parse(a.at));
  return (
    <div className="workspace-content-list">
      {entries.map((entry) => (
        <article key={entry.id}>
          <time dateTime={entry.at}>{new Date(entry.at).toLocaleString(getLocale())}</time>
          <p>{entry.label}</p>
          {entry.detail && <p className="field-hint">{entry.detail}</p>}
        </article>
      ))}
      {!errorsOnly && history.isError && <p role="status">{t("Model activity is unavailable.")}</p>}
      {!entries.length && !history.isError && (
        <p className="field-hint">
          {errorsOnly ? t("No recorded errors") : t("No recorded activity")}
        </p>
      )}
      <p className="field-hint">
        {t("Project events only. Detailed host logs remain local.")}
      </p>
    </div>
  );
}
