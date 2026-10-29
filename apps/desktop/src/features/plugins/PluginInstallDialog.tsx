import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Puzzle, ShieldAlert, ShieldCheck } from "lucide-react";
import { errorText, t } from "../../i18n";
import { Button, Modal } from "../../components/ui";
import { cancelInstall, confirmInstall } from "./api";
import { usePluginUi } from "./pluginUi";
import { permissionText, pluginText } from "./pluginText";

/** Consent window: nothing is installed until the user presses Install. */
export function PluginInstallDialog({ onInstalled }: { onInstalled: (message: string) => void }) {
  const preview = usePluginUi((state) => state.preview);
  const close = usePluginUi((state) => state.showPreview);
  const queryClient = useQueryClient();
  const [working, setWorking] = useState(false);
  const [error, setError] = useState("");
  if (!preview) return null;
  const text = (value: string) => pluginText(preview.translations, value);
  const name = text(preview.manifest.name);
  const dismiss = () => {
    void cancelInstall(preview.token).catch(() => undefined);
    setError("");
    close(null);
  };
  const install = async () => {
    setWorking(true);
    setError("");
    try {
      await confirmInstall(preview.token);
      await queryClient.invalidateQueries({ queryKey: ["plugins"] });
      close(null);
      onInstalled(t("Plugin {{value0}} {{value1}} installed", { value0: name, value1: preview.manifest.version }));
    } catch (failure) {
      setError(errorText(failure));
    } finally {
      setWorking(false);
    }
  };
  const title = preview.isUpdate
    ? t("Update plugin")
    : preview.isDowngrade
      ? t("Install an older plugin version")
      : t("Install plugin");
  return (
    <Modal open onClose={dismiss} title={title} description={t("Check who made the plugin and what it will be allowed to do.")}>
      <div className="plugin-install">
        <div className="plugin-heading">
          <span className="plugin-icon" style={preview.manifest.iconColor ? { background: preview.manifest.iconColor } : undefined}>
            <Puzzle size={18} />
          </span>
          <div>
            <strong>{name}</strong>
            <small>
              {t("Version {{value0}}", { value0: preview.manifest.version })} · {preview.manifest.author.name} (@{preview.manifest.author.github})
            </small>
          </div>
        </div>
        <p>{text(preview.manifest.description)}</p>
        {preview.installedVersion && (
          <p className="field-hint">
            {t("Installed version: {{value0}}", { value0: preview.installedVersion })}
          </p>
        )}
        <p className="field-hint">
          {preview.source.kind === "catalog"
            ? t("Source: FormaCAD plugin catalog, signature and checksum verified.")
            : t("Source: local file. FormaCAD checked the package but nobody reviewed its author.")}
          {preview.source.kind === "catalog" && (
            <>
              {" "}
              <a href={preview.source.pageUrl} target="_blank" rel="noreferrer">
                {t("Plugin page")}
              </a>
            </>
          )}
        </p>
        <h4>{t("Permissions")}</h4>
        {preview.permissions.length === 0 ? (
          <p className="field-hint">{t("The plugin asks for no permissions.")}</p>
        ) : (
          <ul className="plugin-permissions">
            {preview.permissions.map((permission) => {
              const added = preview.addedPermissions.includes(permission) && !!preview.installedVersion;
              return (
                <li key={permission} className={added ? "added" : undefined}>
                  {added ? <ShieldAlert size={14} /> : <ShieldCheck size={14} />}
                  <span>{permissionText(permission)}</span>
                  {added && <em>{t("New")}</em>}
                </li>
              );
            })}
          </ul>
        )}
        {preview.installedVersion && preview.addedPermissions.length > 0 && (
          <p className="plugin-warning" role="status">
            {t("This version asks for new permissions. The current version stays until you agree.")}
          </p>
        )}
        <p className="plugin-hash">{`SHA-256 ${preview.sha256}`}</p>
        {error && <p className="error-inline">{error}</p>}
        <div className="modal-actions">
          <Button onClick={dismiss} disabled={working}>
            {t("Cancel")}
          </Button>
          <Button className="primary" onClick={() => void install()} disabled={working}>
            {preview.isUpdate ? t("Update") : t("Install")}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
