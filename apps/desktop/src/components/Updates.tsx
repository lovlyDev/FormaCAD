import { useCallback, useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Download } from "lucide-react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Button, IconButton, Modal } from "./ui";
import { t, number } from "../i18n";
import { flushPreferences, preferences } from "../lib/persistence";
import { version } from "../../package.json";

const releaseNotesBase = (releaseVersion: string) =>
  `https://github.com/lovlyDev/FormaCAD/blob/v${encodeURIComponent(releaseVersion)}/docs/releases/${encodeURIComponent(releaseVersion)}.md`;

function releaseNotesUrl(url: string, releaseVersion: string): string {
  try {
    const resolved = new URL(url, releaseNotesBase(releaseVersion));
    return ["https:", "http:", "mailto:"].includes(resolved.protocol) ? resolved.href : "";
  } catch {
    return "";
  }
}

function ReleaseNotes({ body, releaseVersion, onOpenError }: { body: string; releaseVersion: string; onOpenError: (error: unknown) => void }) {
  return <div className="update-notes" aria-label={t("updates.releaseNotes")}>
    <Markdown remarkPlugins={[remarkGfm]} urlTransform={url => releaseNotesUrl(url, releaseVersion)}
      components={{ a: ({node: _node, href, children, ...props}) => href
        ? <a {...props} href={href} target="_blank" rel="noopener noreferrer" onClick={event => {
            if (isTauri()) {
              event.preventDefault();
              void openUrl(href).catch(onOpenError);
            }
          }}>{children}</a>
        : <span>{children}</span> }}>
      {body}
    </Markdown>
  </div>;
}

export function Updates({ blocked }: { blocked: boolean }) {
  const [open, setOpen] = useState(false);
  const [update, setUpdate] = useState<Update | null>(null);
  const [status, setStatus] = useState("updates.ready");
  const [working, setWorking] = useState(false);
  const [checking, setChecking] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  const [details, setDetails] = useState("");
  const [autoPrompt, setAutoPrompt] = useState(false);
  const inFlight = useRef(false);
  const resource = useRef<Update | null>(null);
  const blockedRef = useRef(blocked);
  blockedRef.current = blocked;

  const checkNow = useCallback(async (manual: boolean) => {
    if (inFlight.current) return;
    if (manual) setOpen(true);
    inFlight.current = true;
    setChecking(true);
    setDetails("");
    try {
      if (!isTauri()) { setStatus("updates.desktop"); return; }
      const support = await invoke<{configured: boolean; supported: boolean}>("update_support");
      if (!support.configured) { setStatus("updates.unconfigured"); return; }
      // Debian/RPM packages use their package manager; AppImage supports self-update.
      if (!support.supported) { setStatus("updates.packageManager"); return; }
      setStatus("updates.checking");
      const found = await check({ timeout: 15000 });
      await resource.current?.close();
      resource.current = found;
      setUpdate(found);
      setStatus(found ? "updates.available" : "updates.current");
      if (found && !manual) setAutoPrompt(true);
    } catch (error) {
      setStatus("updates.checkFailed");
      setDetails(String(error));
    } finally {
      inFlight.current = false;
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => void checkNow(false), 1500);
    const interval = setInterval(() => void checkNow(false), 6 * 60 * 60 * 1000);
    return () => { clearTimeout(timer); clearInterval(interval); };
  }, [checkNow]);
  useEffect(() => {
    if (autoPrompt && !blocked) { setOpen(true); setAutoPrompt(false); }
  }, [autoPrompt, blocked]);

  async function install() {
    if (!update || blockedRef.current || inFlight.current) return;
    inFlight.current = true;
    setWorking(true);
    setDetails("");
    setStatus("updates.downloading");
    setProgress(null);
    let total = 0, received = 0;
    try {
      await update.download(event => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
          received = 0;
          setProgress(total > 0 ? 0 : null);
        }
        if (event.event === "Progress") received += event.data.chunkLength;
        if (event.event === "Progress" && total > 0) setProgress(Math.min(100, Math.round(received / total * 100)));
        if (event.event === "Finished") setProgress(100);
      }, { timeout: 120000 });
      if (blockedRef.current) { setStatus("updates.busy"); return; }
      setStatus("updates.backup");
      await flushPreferences();
      await invoke("prepare_update", { preferences: preferences() });
      setStatus("updates.installing");
      await update.install();
      await relaunch();
    } catch (error) {
      setStatus("updates.installFailed");
      setDetails(String(error));
    } finally {
      setWorking(false);
      inFlight.current = false;
    }
  }

  return <>
    <IconButton label={update ? t("updates.newVersion", {version: update.version}) : t("updates.title")} active={!!update} onClick={() => {setOpen(true); if (!update) void checkNow(true);}}><Download size={17}/></IconButton>
    <Modal open={open} wide onClose={() => { if (!working) setOpen(false); }} title={t("updates.title")} description={t("updates.installed", {version})}>
      <div className="update-summary" role="status">
        <span className="update-summary-icon"><Download size={18}/></span>
        <div><strong>{update ? t("updates.newVersion", {version: update.version}) : t(status)}</strong>
          {update && <span>{t(status)}</span>}
        </div>
      </div>
      {update?.body && <ReleaseNotes body={update.body} releaseVersion={update.version} onOpenError={error => setDetails(String(error))} />}
      {working && status === "updates.downloading" && <div className="update-progress">
        <div className="update-progress-heading">
          <span>{t("updates.downloadProgress")}</span>
          <strong>{progress === null ? t("updates.pleaseWait") : `${number(progress, 0)}%`}</strong>
        </div>
        <div className={`update-progress-track${progress === null ? " indeterminate" : ""}`}
          role="progressbar" aria-label={t("updates.downloadProgress")}
          aria-valuemin={0} aria-valuemax={100} aria-valuenow={progress ?? undefined}
          aria-valuetext={progress === null ? t("updates.pleaseWait") : t("updates.progress", {percent: number(progress, 0)})}>
          <div className="update-progress-fill" style={progress === null ? undefined : {width: `${progress}%`}} />
        </div>
      </div>}
      {update && <p className="field-hint">{t("updates.preserve")}</p>}
      {blocked && <p className="field-hint">{t("updates.busy")}</p>}
      {details && <details className="update-details"><summary>{t("updates.details")}</summary><pre>{details}</pre></details>}
      <div className="modal-actions">
        <Button disabled={working} onClick={() => setOpen(false)}>{t("updates.later")}</Button>
        {update ? <Button className="primary" disabled={working || blocked || checking} onClick={() => void install()}>{t("updates.install")}</Button> : <Button disabled={checking || working} onClick={() => void checkNow(true)}>{t("updates.check")}</Button>}
      </div>
    </Modal>
  </>;
}
