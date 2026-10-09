import { useState } from "react";
import { LockKeyhole, RefreshCw, Copy, LoaderCircle } from "lucide-react";
import { Button } from "../../components/ui";
import { t, errorText } from "../../i18n";
import type { Project } from "../../types";
import { copyProjectForEditing } from "./projectAccessApi";
import "./ProjectAccessNotice.css";

export function ProjectAccessNotice({ projectId, checking, error, writable, retry, onCopy, onError }: {
  projectId: string; checking: boolean; error?: string; writable: boolean; retry: () => void; onCopy: (project: Project) => void; onError: (message: string) => void;
}) {
  const [copying, setCopying] = useState(false);
  if (writable) return null;
  async function copy() {
    if (copying) return; setCopying(true);
    try { onCopy(await copyProjectForEditing(projectId)); }
    catch (cause) { onError(errorText(cause)); }
    finally { setCopying(false); }
  }
  return <div className="project-access-notice" role="status">
    {checking ? <LoaderCircle size={16} className="spin" /> : <LockKeyhole size={16} />}
    <span>{checking ? t("Checking project access…") : error ?? t("Read-only: this project is open for editing in another instance.")}</span>
    {!checking && <div><Button disabled={copying} onClick={retry}><RefreshCw size={14} />{t("Retry editing access")}</Button><Button disabled={copying} onClick={() => void copy()}><Copy size={14} />{t("Save an editable copy")}</Button></div>}
  </div>;
}
