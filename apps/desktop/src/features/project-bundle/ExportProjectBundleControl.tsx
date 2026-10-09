import { useState } from "react";
import { Download, ShieldCheck } from "lucide-react";
import { Button, Checkbox, IconButton, Modal } from "../../components/ui";
import { t } from "../../i18n";
import type { Project } from "../../types";
import "../model-editor/EditorDialog.css";
import "./ExportProjectBundle.css";

export function ExportProjectBundleControl({ project, onExport }: { project: Project; onExport: (project: Project, redactConversation: boolean) => void }) {
  const [open, setOpen] = useState(false), [redact, setRedact] = useState(false);
  return <><IconButton label={t("Export project bundle")} onClick={() => setOpen(true)}><Download size={14} /></IconButton>
    <Modal open={open} onClose={() => setOpen(false)} title={t("Export project bundle")} description={project.name} wide className="model-editor-modal project-bundle-modal">
      <div className="model-editor-scroll project-bundle-review">
        <Checkbox checked={redact} onChange={setRedact}>{t("Exclude conversation and activity text")}</Checkbox>
        <p className="field-hint">{redact ? t("The bundle excludes chat messages, revision prompts, export records and thumbnails. Model geometry, source code, names, parameters and required files remain included and may contain private information.") : t("The full bundle includes the model, files, saved revisions, model undo history and conversation.")}</p>
        {redact && <p className="field-hint">{t("For legacy scripts, all attachments are retained because script file dependencies cannot be identified safely.")}</p>}
      </div>
      <footer className="model-editor-footer"><small><ShieldCheck size={14} />{t("Exporting creates a copy and preserves this project.")}</small><div className="modal-actions">
        <Button onClick={() => setOpen(false)}>{t("Cancel")}</Button><Button className="primary" onClick={() => { setOpen(false); onExport(project, redact); }}><Download size={15} />{t("Export")}</Button>
      </div></footer>
    </Modal>
  </>;
}
