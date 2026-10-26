import "./plugins.css";
import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { FileUp, LoaderCircle, Puzzle, RefreshCw, ShieldCheck, Trash2 } from "lucide-react";
import { errorText, getLocale, t } from "../../i18n";
import { Button, Checkbox, IconButton, Select } from "../../components/ui";
import { native } from "../../lib/api";
import {
  installFromFile,
  pluginJournal,
  savePluginSettings,
  setActiveVersion,
  setPluginEnabled,
  uninstallPlugin,
  type FieldValue,
  type PluginSummary,
} from "./api";
import { formValid, PluginForm } from "./PluginForm";
import { usePluginUi } from "./pluginUi";
import { initialValues, permissionText, pluginText } from "./pluginText";
import { usePlugins } from "./usePluginCommands";

/** Settings → Plugins: installed plugins, versions, permissions, settings. */
export function PluginsSettings({ open }: { open: boolean }) {
  const plugins = usePlugins(open);
  const queryClient = useQueryClient();
  const showPreview = usePluginUi((state) => state.showPreview);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  if (!native) return <p>{t("Plugins are available in the desktop app.")}</p>;
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["plugins"] });
  const act = async (work: () => Promise<unknown>) => {
    setBusy(true);
    setError("");
    try {
      await work();
      await refresh();
    } catch (failure) {
      setError(errorText(failure));
    } finally {
      setBusy(false);
    }
  };
  return (
    <>
      <div className="settings-section-title">
        <h3>{t("Plugins")}</h3>
        <IconButton label={t("Refresh plugin list")} onClick={() => void plugins.refetch()}>
          <RefreshCw size={14} />
        </IconButton>
      </div>
      <p>
        {t("Plugins run in a sandbox and can do only what you allowed. Installed plugins work offline.")}
      </p>
      <Button
        disabled={busy}
        onClick={() =>
          void act(async () => {
            const preview = await installFromFile(t("Install plugin from file"), t("FormaCAD plugin"));
            if (preview) showPreview(preview);
          })
        }
      >
        <FileUp size={14} /> {t("Install from file…")}
      </Button>
      {error && <p className="error-inline">{error}</p>}
      {plugins.isFetching && <LoaderCircle size={16} className="spin" />}
      {plugins.error && <p className="error-inline">{errorText(plugins.error)}</p>}
      {plugins.data?.length === 0 && <p>{t("No plugins installed.")}</p>}
      <div className="plugin-list">
        {plugins.data?.map((plugin) => (
          <PluginCard key={plugin.manifest.id} plugin={plugin} busy={busy} act={act} />
        ))}
      </div>
    </>
  );
}

function PluginCard({
  plugin,
  busy,
  act,
}: {
  plugin: PluginSummary;
  busy: boolean;
  act: (work: () => Promise<unknown>) => Promise<void>;
}) {
  const text = (value: string) => pluginText(plugin.translations, value);
  const id = plugin.manifest.id;
  const name = text(plugin.manifest.name);
  const settingsFields = plugin.manifest.contributes.settings;
  const [values, setValues] = useState<Record<string, FieldValue>>(() => initialValues(settingsFields, plugin.settings));
  const [confirmRemove, setConfirmRemove] = useState(false);
  const [showJournal, setShowJournal] = useState(false);
  const journal = useQuery({ queryKey: ["plugin-journal", id], queryFn: () => pluginJournal(id), enabled: showJournal, staleTime: 0 });
  return (
    <section className="plugin-card" aria-label={name}>
      <div className="plugin-heading">
        <span className="plugin-icon" style={plugin.manifest.iconColor ? { background: plugin.manifest.iconColor } : undefined}>
          <Puzzle size={16} />
        </span>
        <div>
          <strong>{name}</strong>
          <small>
            {t("Version {{value0}}", { value0: plugin.active })} · {plugin.manifest.author.name}
          </small>
        </div>
        <Checkbox checked={plugin.enabled} disabled={busy} onChange={(enabled) => void act(() => setPluginEnabled(id, enabled))}>
          {t("Enabled")}
        </Checkbox>
      </div>
      <p>{text(plugin.manifest.description)}</p>
      {plugin.problem && <p className="error-inline">{errorText(JSON.stringify({ code: plugin.problem, message: plugin.problem }))}</p>}
      {plugin.versions.length > 1 && (
        <label>
          {t("Active version")}
          <Select
            aria-label={t("Active version")}
            value={plugin.active}
            disabled={busy}
            onChange={(event) => void act(() => setActiveVersion(id, event.target.value, plugin.acceptedPermissions))}
          >
            {plugin.versions.map((version) => (
              <option key={version.version} value={version.version}>
                {version.version}
              </option>
            ))}
          </Select>
        </label>
      )}
      <h4>{t("Permissions")}</h4>
      {plugin.acceptedPermissions.length === 0 ? (
        <p className="field-hint">{t("The plugin asks for no permissions.")}</p>
      ) : (
        <ul className="plugin-permissions">
          {plugin.acceptedPermissions.map((permission) => (
            <li key={permission}>
              <ShieldCheck size={14} />
              <span>{permissionText(permission)}</span>
            </li>
          ))}
        </ul>
      )}
      {settingsFields.length > 0 && (
        <>
          <h4>{t("Plugin settings")}</h4>
          <PluginForm fields={settingsFields} values={values} onChange={setValues} translations={plugin.translations} disabled={busy} />
          <Button disabled={busy || !formValid(settingsFields, values)} onClick={() => void act(() => savePluginSettings(id, values))}>
            {t("Save settings")}
          </Button>
        </>
      )}
      <div className="plugin-actions">
        <Button onClick={() => setShowJournal(!showJournal)}>{showJournal ? t("Hide journal") : t("Show journal")}</Button>
        {confirmRemove ? (
          <>
            <Button onClick={() => setConfirmRemove(false)}>{t("Cancel")}</Button>
            <Button className="danger" disabled={busy} onClick={() => void act(() => uninstallPlugin(id))}>
              <Trash2 size={14} /> {t("Remove plugin and its data")}
            </Button>
          </>
        ) : (
          <Button disabled={busy} onClick={() => setConfirmRemove(true)}>
            <Trash2 size={14} /> {t("Remove")}
          </Button>
        )}
      </div>
      {showJournal && (
        <div className="plugin-journal">
          {journal.data?.length === 0 && <p className="field-hint">{t("No journal entries.")}</p>}
          {journal.data?.slice(0, 20).map((entry, index) => (
            <small key={index} className={entry.ok ? undefined : "error-inline"}>
              {new Date(entry.at).toLocaleString(getLocale())} · {entry.version ?? ""} · {t(`plugins.event.${entry.event}`)}
              {entry.durationMs !== undefined ? ` · ${entry.durationMs} ms` : ""} · {entry.detail}
            </small>
          ))}
        </div>
      )}
    </section>
  );
}
