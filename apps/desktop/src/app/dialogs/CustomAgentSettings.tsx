import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "../../components/ui";
import { errorText, t } from "../../i18n";
import {
  getCustomAgentConfig,
  native,
  setCustomAgentConfig,
} from "../../lib/api";
import "./custom-agent.css";

export function CustomAgentSettings() {
  const client = useQueryClient();
  const current = useQuery({
    queryKey: ["custom-agent"],
    queryFn: getCustomAgentConfig,
    enabled: native,
  });
  const [executable, setExecutable] = useState("");
  const [argumentsText, setArgumentsText] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    setExecutable(current.data?.executable ?? "");
    setArgumentsText(current.data?.args.join("\n") ?? "");
  }, [current.data]);

  if (!native) return null;

  const save = async () => {
    setSaving(true);
    setError("");
    setSaved(false);
    try {
      await setCustomAgentConfig({
        executable: executable.trim(),
        args: argumentsText
          .split("\n")
          .map((arg) => arg.trim())
          .filter(Boolean),
      });
      await Promise.all([
        client.invalidateQueries({ queryKey: ["custom-agent"] }),
        client.invalidateQueries({ queryKey: ["health"] }),
      ]);
      setSaved(true);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="custom-agent-settings">
      <h3>{t("Custom CLI")}</h3>
      <p className="field-hint">
        {t("The executable receives the request, recent chat, and current model on stdin. It must return one JSON result on stdout.")}
      </p>
      <label>
        {t("Executable path")}
        <div className="custom-agent-path">
          <input
            value={executable}
            onChange={(event) => setExecutable(event.target.value)}
            aria-label={t("Executable path")}
          />
          <Button
            onClick={async () => {
              const selected = await open({ multiple: false, directory: false });
              if (typeof selected === "string") setExecutable(selected);
            }}
          >
            {t("Browse")}
          </Button>
        </div>
      </label>
      <label>
        {t("Arguments, one per line")}
        <textarea
          value={argumentsText}
          onChange={(event) => setArgumentsText(event.target.value)}
          aria-label={t("Arguments, one per line")}
          rows={3}
        />
      </label>
      <div className="custom-agent-actions">
        <Button disabled={saving || !executable.trim()} onClick={() => void save()}>
          {saving ? t("Saving…") : t("Save Custom CLI")}
        </Button>
        {saved && <span>{t("Custom CLI saved")}</span>}
      </div>
      {error && <p className="error-inline">{error}</p>}
    </div>
  );
}
