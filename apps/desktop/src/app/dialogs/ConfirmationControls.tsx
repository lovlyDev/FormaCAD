import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { t, errorText } from "../../i18n";
import { Checkbox, Select } from "../../components/ui";
import { confirmationSettings, saveConfirmationSettings, needsConfirmation, type ConfirmationSettings } from "../../lib/api";
export function ConfirmationControls() {
  const query = useQuery({
    queryKey: ["confirmations"],
    queryFn: confirmationSettings,
  });
  const client = useQueryClient();
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const value = query.data;
  async function save(next: ConfirmationSettings) {
    setSaving(true);
    try {
      await saveConfirmationSettings(next);
      client.setQueryData(["confirmations"], next);
      setError("");
    } catch (error) {
      setError(errorText(error));
    } finally {
      setSaving(false);
    }
  }
  if (!value)
    return (
      <p>{query.error ? errorText(query.error) : t("Loading settings…")}</p>
    );
  return (
    <fieldset disabled={saving} className="confirmation-controls">
      <h3>{t("Confirmations")}</h3>
      <Select
        aria-label={t("Confirmation mode")}
        value={value.mode}
        onChange={(e) =>
          void save({
            mode: e.target.value as ConfirmationSettings["mode"],
            overrides: {},
          })
        }
      >
        <option value="all">{t("Confirm everything")}</option>
        <option value="cli">{t("Only CLI and dependencies")}</option>
        <option value="none">{t("No confirmations")}</option>
      </Select>
      <p className="field-hint">{t("Customize individual actions below.")}</p>
      {Object.entries({
        run_agent: "Connect to CLI agent",
        modify_project: "Edit models and project files",
        convert_file: "Convert CAD files",
        export_file: "Export files",
        install_dependency: "Install dependencies",
      }).map(([action, label]) => (
        <Checkbox
          key={action}
          checked={needsConfirmation(value, action)}
          onChange={(checked) =>
            void save({
              ...value,
              overrides: { ...value.overrides, [action]: checked },
            })
          }
        >
          {t(label)}
        </Checkbox>
      ))}
      {error && <p className="error-inline">{errorText(error)}</p>}
    </fieldset>
  );
}
