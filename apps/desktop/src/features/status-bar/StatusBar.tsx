import { GitBranch } from "lucide-react";
import { t, quantity, systemText } from "../../i18n";
export function StatusBar({
  busy,
  chatBusy,
  stage,
  native,
  revisions,
  units,
  version,
}: {
  busy: boolean;
  chatBusy: boolean;
  stage: string;
  native: boolean;
  revisions: number | null;
  units?: string;
  version: string;
}) {
  const unit = units ?? "mm";
  return (
    <footer className="statusbar">
      <span className="status-dot" />
      <span role="status">
        {busy
          ? chatBusy
            ? t("Agent working")
            : systemText(stage)
          : native
            ? t("Local workspace ready")
            : t("Browser workspace · desktop required for AI")}
      </span>
      {revisions !== null && (
        <>
          <span className="status-separator" />
          <GitBranch size={11} />
          <span className="revision-badge">
            {revisions
              ? t("Revision {{value0}}", { value0: revisions })
              : t("No revisions")}
          </span>
          <span>{quantity("revisions", revisions)}</span>
          <span>{t(unit)}</span>
          <span className="workspace-key-hint">
            {t("F6: next area · Ctrl+Space: maximize · T/N: viewport panels")}
          </span>
        </>
      )}
      <span className="statusbar-spacer" />
      <span>v{version}</span>
    </footer>
  );
}
