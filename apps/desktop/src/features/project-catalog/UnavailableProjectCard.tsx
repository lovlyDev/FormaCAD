import "./UnavailableProjectCard.css";
import { AlertCircle } from "lucide-react";
import { Button } from "../../components/ui";
import { t } from "../../i18n";
import type { UnavailableEntry } from "./useProjectCatalog";

const statusText = {
  recovery_required: "This project requires recovery. No automatic repair was performed.",
  changed: "This project changed while it was being read. Refresh the catalog.",
  unavailable: "This project could not be verified. Its saved files were not changed.",
  limit: "This project exceeds the catalog response budget. Its saved files were not changed.",
} as const;

export function UnavailableProjectCard({ entry, onRetry }: { entry: UnavailableEntry; onRetry: () => void }) {
  return <article className="project-card project-card-unavailable" aria-label={t("Unavailable project")} data-row-key={entry.rowKey}>
    <div className="project-art"><AlertCircle size={48} aria-hidden="true" /></div>
    <div className="project-card-info">
      <div><strong>{entry.index.name ?? t("Unavailable project")}</strong><p>{t(statusText[entry.issue.status])}</p>
        <Button onClick={onRetry}>{t("Refresh project catalog")}</Button>
        <details><summary>{t("Technical details")}</summary><pre>{entry.issue.code}<br />{entry.issue.details}</pre></details>
      </div>
    </div>
  </article>;
}
