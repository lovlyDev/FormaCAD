import { useRef, useState } from "react";
import { FolderOpen, FolderInput, ShieldCheck, LoaderCircle } from "lucide-react";
import { Button, Checkbox, Modal } from "../../components/ui";
import { t, errorText, number } from "../../i18n";
import type { Project } from "../../types";
import { inspectProjectFolder, importProjectFolder, type ProjectFolderInspection } from "./projectFolderApi";
import "../model-editor/EditorDialog.css";
import "./ImportProjectFolder.css";

export function ImportProjectFolderButton({ onImported, onError }: { onImported: (project: Project) => void; onError: (message: string) => void }) {
  const [folder, setFolder] = useState<ProjectFolderInspection | null>(null);
  const [open, setOpen] = useState(false), [busy, setBusy] = useState(false), [saveCopy, setSaveCopy] = useState(false), [error, setError] = useState<string | null>(null);
  const inFlight = useRef(false);
  async function inspect() {
    if (inFlight.current) return;
    inFlight.current = true; setBusy(true);
    try {
      const result = await inspectProjectFolder();
      if (result) { setFolder(result); setSaveCopy(result.identityExists); setError(null); setOpen(true); }
    } catch (cause) { onError(errorText(cause)); }
    finally { inFlight.current = false; setBusy(false); }
  }
  async function accept() {
    if (!folder || inFlight.current || (folder.identityExists && !saveCopy)) return;
    inFlight.current = true; setBusy(true); setError(null);
    try { const result = await importProjectFolder(folder, saveCopy); setOpen(false); onImported(result); }
    catch (cause) { setError(errorText(cause)); }
    finally { inFlight.current = false; setBusy(false); }
  }
  return <>
    <Button onClick={() => void inspect()} disabled={busy}><FolderOpen size={15} />{t("Import project folder")}</Button>
    <Modal open={open} onClose={() => { if (!busy) setOpen(false); }} wide className="model-editor-modal project-folder-modal" title={t("Import project folder")} description={t("Review this project before importing it into your local workspace.")}>
      <div className="model-editor-scroll project-folder-review">
        <div className="project-folder-heading"><FolderOpen size={20} /><strong>{folder?.name}</strong><span>{t("Saved revisions: {{value0}}", { value0: number(folder?.revisionCount ?? 0, 0) })}</span></div>
        <label className="project-folder-source">{t("Source folder")}<code>{folder?.path}</code></label>
        <p className="field-hint">{t("The selected folder stays unchanged. A checked copy of its model, files and model history is saved in this application's project storage.")}</p>
        <Checkbox checked={saveCopy} disabled={busy || !!folder?.identityExists} onChange={setSaveCopy}>{t("Create a copy with a new project ID")}</Checkbox>
        {folder?.identityExists && <p className="field-hint">{t("This project ID already exists locally. Importing creates a separate copy and preserves the existing project.")}</p>}
        {error && <p className="error-inline" role="alert">{error}</p>}
      </div>
      <footer className="model-editor-footer"><small><ShieldCheck size={14} />{t("File checksums are verified before importing.")}</small><div className="modal-actions">
        <Button onClick={() => setOpen(false)} disabled={busy}>{t("Cancel")}</Button>
        <Button className="primary" onClick={() => void accept()} disabled={busy || !folder}>{busy ? <LoaderCircle size={15} className="spin" /> : <FolderInput size={15} />}{t("Import project")}</Button>
      </div></footer>
    </Modal>
  </>;
}
