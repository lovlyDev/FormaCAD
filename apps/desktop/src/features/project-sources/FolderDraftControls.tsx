import { invoke } from "@tauri-apps/api/core";
import { Archive, FileText } from "lucide-react";
import { useState } from "react";
import { Button } from "../../components/ui";
import { errorText, t } from "../../i18n";
import { isFolderWriteOpeningCurrent } from "./folderWriteResult";
import type { FolderWriteOpening } from "./useFolderWriteAction";
import "./FolderDraftControls.css";

type Props = {
  opening: FolderWriteOpening;
  edited: boolean;
  editorText: string;
  editingDisabled: boolean;
  useDraft: (editorText: string) => void;
};

export function FolderDraftControls({ opening, edited, editorText, editingDisabled, useDraft }: Props) {
  const [busy, setBusy] = useState(false);
  const [draft, setDraft] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const source = {
    projectId: opening.projectId,
    sourceId: opening.source.sourceId,
    expectedCommitId: opening.source.commitId,
  };

  async function run(action: "keep" | "review") {
    if (busy || !isFolderWriteOpeningCurrent(opening)) return;
    setBusy(true);
    setStatus("");
    try {
      if (action === "keep") {
        await invoke<void>("keep_folder_editor_draft", { source, editorText });
        if (isFolderWriteOpeningCurrent(opening)) setStatus(t("Draft kept on this device."));
      } else {
        const found = await invoke<string | null>("review_folder_editor_draft", { source });
        if (!isFolderWriteOpeningCurrent(opening)) return;
        setDraft(found);
        if (found === null) setStatus(t("No local draft for this saved revision."));
      }
    } catch (cause) {
      if (isFolderWriteOpeningCurrent(opening)) setStatus(errorText(cause));
    } finally {
      setBusy(false);
    }
  }

  return <section className="folder-draft-controls">
    <div className="modal-actions">
      <Button disabled={busy || !edited} onClick={() => void run("keep")}>
        <Archive size={14} />{t("Keep draft on this device")}
      </Button>
      <Button disabled={busy} onClick={() => void run("review")}>
        <FileText size={14} />{t("Review local draft")}
      </Button>
    </div>
    {status && <p className="field-hint" role="status">{status}</p>}
    {draft !== null && <>
      <textarea className="program-editor" aria-label={t("Local draft text")} readOnly value={draft} />
      <div className="modal-actions">
        <Button onClick={() => setDraft(null)}>{t("Cancel")}</Button>
        <Button disabled={editingDisabled} onClick={() => {
          if (!isFolderWriteOpeningCurrent(opening)) return;
          useDraft(draft);
          setDraft(null);
        }}>{t("Replace editor text with draft")}</Button>
      </div>
    </>}
  </section>;
}
