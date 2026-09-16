import { useCallback, useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { Download } from "lucide-react";
import { Button, IconButton, Modal } from "./ui";
import { t, number } from "../i18n";
import { flushPreferences, preferences } from "../lib/persistence";
import { version } from "../../package.json";

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
    let total = 0, received = 0;
    try {
      await update.download(event => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") received += event.data.chunkLength;
        if (total) setProgress(Math.min(100, Math.round(received / total * 100)));
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
    <Modal open={open} onClose={() => { if (!working) setOpen(false); }} title={t("updates.title")} description={t("updates.installed", {version})}>
      <p role="status">{t(status)}</p>
      {update && <p className="field-hint">{t("updates.newVersion", {version: update.version})}</p>}
      {update?.body && <pre className="update-notes">{update.body}</pre>}
      {working && <p>{progress === null ? t("updates.pleaseWait") : t("updates.progress", {percent: number(progress, 0)})}</p>}
      {update && <p className="field-hint">{t("updates.preserve")}</p>}
      {blocked && <p className="field-hint">{t("updates.busy")}</p>}
      {details && <details><summary>{t("updates.details")}</summary><pre className="update-notes">{details}</pre></details>}
      <div className="modal-actions">
        <Button disabled={working} onClick={() => setOpen(false)}>{t("updates.later")}</Button>
        {update ? <Button className="primary" disabled={working || blocked || checking} onClick={() => void install()}>{t("updates.install")}</Button> : <Button disabled={checking || working} onClick={() => void checkNow(true)}>{t("updates.check")}</Button>}
      </div>
    </Modal>
  </>;
}
