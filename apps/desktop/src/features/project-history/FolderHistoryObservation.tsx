import { Button } from "../../components/ui";
import { t } from "../../i18n";

export function FolderHistoryObservation({ observation, busy, onCheck }: {
  observation?: "historical" | "absent";
  busy: boolean;
  onCheck: () => void;
}) {
  const message = observation === "historical"
    ? t("The commit is saved in history. Reopen the folder to load its current model.")
    : observation === "absent"
      ? t("The commit was not found. Reopen the folder before making further changes.")
      : t("The result of the last model history action could not be confirmed.");
  return <div className="error-inline">
    <span role="status">{message}</span>
    <Button disabled={busy} onClick={onCheck}>{t("Check saved commit")}</Button>
  </div>;
}
