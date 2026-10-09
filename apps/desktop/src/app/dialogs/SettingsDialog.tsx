import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Check, Cpu, LoaderCircle, RefreshCw, ShieldCheck } from "lucide-react";
import { t, errorText, systemText, getLocale, setLocale } from "../../i18n";
import { Button, IconButton, Modal, Select } from "../../components/ui";
import { applyTheme } from "../../lib/theme";
import { usePersistentState } from "../../lib/persistence";
import { health, permissionAudit, chooseCadPython, native } from "../../lib/api";
import type { Agent, Project } from "../../types";
import { ConfirmationControls } from "./ConfirmationControls";
import { CustomAgentSettings } from "./CustomAgentSettings";
export function SettingsDialog({
  open,
  close,
  project,
  onAgent,
}: {
  open: boolean;
  close: () => void;
  project: Project | null;
  onAgent: (a: Agent) => Promise<void>;
}) {
  const system = useQuery({
    queryKey: ["health"],
    queryFn: health,
    enabled: open,
    staleTime: 30000,
  });
  const [page, setPage] = usePersistentState("forma.ui.settingsPage", "AI agents");
  const [configuring, setConfiguring] = useState(false);
  const [configurationError, setConfigurationError] = useState("");
  const audit = useQuery({
    queryKey: ["permission-audit"],
    queryFn: permissionAudit,
    enabled: open && page === "Permissions",
    staleTime: 0,
  });
  return (
    <Modal
      open={open}
      onClose={close}
      title={t("Settings")}
      description={t("Application preferences")}
      wide
    >
      <div className="settings-layout">
        <nav>
          {[
            "AI agents",
            "CAD environment",
            "Permissions",
            "Appearance",
            "Privacy",
          ].map((s) => (
            <button
              className={page === s ? "active" : ""}
              onClick={() => setPage(s)}
              key={s}
            >
              {t(s)}
            </button>
          ))}
        </nav>
        <section>
          {page === "AI agents" || page === "CAD environment" ? (
            <>
              <div className="settings-section-title">
                <h3>{t(page)}</h3>
                <IconButton
                  label={t("Refresh environment")}
                  onClick={() => void system.refetch()}
                >
                  <RefreshCw size={14} />
                </IconButton>
              </div>
              {system.isFetching && <LoaderCircle size={17} className="spin" />}
              {system.error && (
                <p className="error-inline">{errorText(system.error)}</p>
              )}
              {system.data
                ?.filter((h) =>
                  page === "AI agents"
                    ? /codex|claude|custom cli|desktop runtime/i.test(h.name)
                    : !/codex|claude|custom cli|desktop runtime/i.test(h.name),
                )
                .map((h) => (
                  <div className="health-row" key={h.name}>
                    <span
                      className={`health-icon ${h.available ? "connected" : ""}`}
                    >
                      {h.available ? <Check size={15} /> : <Cpu size={15} />}
                    </span>
                    <div>
                      <strong>{t(h.name)}</strong>
                      <small>{systemText(h.detail)}</small>
                    </div>
                    <span>
                      {h.available ? t("Detected") : t("Unavailable")}
                    </span>
                  </div>
                ))}
              {native && page === "CAD environment" && (
                <>
                  <Button
                    className="cad-python-action"
                    disabled={configuring}
                    onClick={async () => {
                      setConfiguring(true);
                      setConfigurationError("");
                      try {
                        await chooseCadPython();
                        await system.refetch();
                      } catch (error) {
                        setConfigurationError(errorText(error));
                      } finally {
                        setConfiguring(false);
                      }
                    }}
                  >
                    {configuring
                      ? t("Проверка CadQuery…")
                      : t("Выбрать Python с CadQuery")}
                  </Button>
                  {configurationError && (
                    <p className="error-inline">{configurationError}</p>
                  )}
                </>
              )}
              {project && page === "AI agents" && (
                <label>
                  {t("Project agent")}
                  <Select
                    value={project.agent}
                    onChange={(e) => void onAgent(e.target.value as Agent)}
                  >
                    <option value="codex">OpenAI Codex</option>
                    <option value="claude">Claude Code</option>
                    <option value="custom">{t("Custom CLI")}</option>
                  </Select>
                </label>
              )}
              {page === "AI agents" && <CustomAgentSettings />}
              <p className="field-hint">
                {page === "AI agents"
                  ? t("Uses your existing CLI login.")
                  : system.data?.some((item) => item.name === "OpenCascade kernel" && item.available)
                    ? t("OpenCascade handles STEP import and CAD modeling. Python and CadQuery are optional for legacy workflows.")
                    : t("Python and CadQuery power STEP import and export.")}
              </p>
            </>
          ) : page === "Permissions" ? (
            <>
              <ConfirmationControls />
              <div className="settings-section-title">
                <h3>{t("Recent decisions")}</h3>
                <IconButton
                  label={t("Refresh audit log")}
                  onClick={() => void audit.refetch()}
                >
                  <RefreshCw size={14} />
                </IconButton>
              </div>
              {audit.isFetching && <LoaderCircle className="spin" size={16} />}
              {audit.error && (
                <p className="error-inline">{errorText(audit.error)}</p>
              )}
              {!native && (
                <p>
                  {t("The permission journal is available in the desktop app.")}
                </p>
              )}
              {native && audit.data?.length === 0 && (
                <p>{t("No permission requests recorded.")}</p>
              )}
              <div style={{ maxHeight: 320, overflowY: "auto" }}>
                {audit.data?.map((entry) => (
                  <div className="health-row" key={entry.id}>
                    <ShieldCheck size={16} />
                    <div>
                      <strong>
                        {t(entry.action)} · {t(entry.decision)}
                      </strong>
                      <small>{systemText(entry.detail)}</small>
                      <small>
                        {new Date(entry.createdAt).toLocaleString(getLocale())}{" "}
                        · {entry.projectId.slice(0, 8)}
                      </small>
                    </div>
                  </div>
                ))}
              </div>
              <p className="field-hint">
                {t(
                  "Latest 100 recorded decisions. An allowed decision does not mean the operation completed. Grants expire and cannot be reused.",
                )}
              </p>
            </>
          ) : page === "Appearance" ? (
            <>
              <label>
                {t("Language")}
                <Select
                  aria-label={t("Language")}
                  value={getLocale()}
                  onChange={(event) =>
                    setLocale(event.target.value as "en" | "ru")
                  }
                >
                  <option value="ru">Русский</option>
                  <option value="en">English</option>
                </Select>
              </label>
              <h3>{t("Appearance")}</h3>
              <label>
                {t("Theme")}
                <Select
                  defaultValue={localStorage.getItem("forma.theme") ?? "dark"}
                  onChange={(e) => {
                    applyTheme(e.target.value);
                  }}
                >
                  <option value="dark">{t("Dark")}</option>
                  <option value="light">{t("Light")}</option>
                </Select>
              </label>
              <p>
                {t(
                  "The 3D viewport and controls follow the selected theme.",
                )}
              </p>
            </>
          ) : (
            <>
              <ShieldCheck size={28} />
              <h3>{t("Your work stays yours.")}</h3>
              <p>
                {t(
                  "No developer backend and no analytics. Projects are saved locally. AI prompts go to your selected agent’s provider only when you approve a connection.",
                )}
              </p>
              <p>
                {t(
                  "Browser projects remain in browser storage. Clear site data only after exporting your work.",
                )}
              </p>
            </>
          )}
        </section>
      </div>
    </Modal>
  );
}
