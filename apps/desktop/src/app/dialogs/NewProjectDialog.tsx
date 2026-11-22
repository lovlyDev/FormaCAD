import { useRef, useState } from "react";
import { LoaderCircle, Plus, FileBox } from "lucide-react";
import { t } from "../../i18n";
import { Button, Modal, Select } from "../../components/ui";
import { newProject } from "../../stores/workspace";
import type { Agent, Project } from "../../types";
import type { BlankFolderDraft } from "../../features/project-sources/createFolderSelection";
import type { StepFolderDraft } from "../../features/project-sources/stepFolderSelection";
export function NewProjectDialog({
  open,
  close,
  onCreate,
  onCreateFolder,
  onTransferStep,
  importNames = [],
}: {
  importNames?: string[];
  open: boolean;
  close: () => void;
  onCreate: (p: Project) => Promise<void>;
  onCreateFolder?: (draft: BlankFolderDraft) => Promise<void>;
  onTransferStep?: (draft: StepFolderDraft) => Promise<void>;
}) {
  const [name, setName] = useState(t("Untitled part"));
  const [units, setUnits] = useState<Project["units"]>("mm");
  const [agent, setAgent] = useState<Agent>("codex");
  const [saving, setSaving] = useState(false);
  const inFlight = useRef(false);
  function submit(save: () => Promise<void>) {
    if (!name.trim() || inFlight.current) return;
    inFlight.current = true;
    setSaving(true);
    void save().finally(() => {
      inFlight.current = false;
      setSaving(false);
    });
  }
  return (
    <Modal
      open={open}
      onClose={() => { if (!inFlight.current) close(); }}
      title={importNames.length ? t("Create a project from selected files") : t("Make room for an idea")}
      description={importNames.length ? t("Selected models will be imported. STEP files need local conversion before they appear in the scene.") : t("Start with an empty workspace.")}
    >
      {importNames.length > 0 && <ul className="import-file-list">{importNames.map(name => <li key={name}><FileBox size={15}/><span>{name}</span></li>)}</ul>}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          submit(() => onCreate(newProject(name, "blank", units, agent)));
        }}
      >
        <label>
          {t("Project name")}
          <input
            required
            maxLength={80}
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <div className="form-row">
          <label>
            {t("Display units")}
            <Select
              value={units}
              onChange={(e) => setUnits(e.target.value as Project["units"])}
            >
              <option value="mm">{t("Millimeters")}</option>
              <option value="cm">{t("Centimeters")}</option>
              <option value="inch">{t("Inches")}</option>
            </Select>
          </label>
          <label>
            {t("AI agent")}
            <Select
              value={agent}
              onChange={(e) => setAgent(e.target.value as Agent)}
            >
              <option value="codex">OpenAI Codex</option>
              <option value="claude">Claude Code</option>
              <option value="custom">{t("Custom CLI")}</option>
            </Select>
          </label>
        </div>
        {onTransferStep && importNames.length === 0 && <small>
          {t("STEP transfer creates a new project in millimeters with imported geometry. The original project and its files stay unchanged. Editing history is not transferred.")}
        </small>}
        <div className="modal-actions">
          <Button type="button" onClick={close} disabled={saving}>
            {t("Cancel")}
          </Button>
          {onCreateFolder && importNames.length === 0 && <Button
            type="button" disabled={saving || !name.trim()}
            onClick={() => submit(() => onCreateFolder({ name, units, agent }))}
          >
            {t("Create in folder")}
          </Button>}
          {onTransferStep && importNames.length === 0 && <Button
            type="button" disabled={saving || !name.trim()}
            onClick={() => submit(() => onTransferStep({ name, agent }))}
          >
            {t("Transfer STEP to folder")}
          </Button>}
          <Button
            className="primary"
            type="submit"
            disabled={saving || !name.trim()}
          >
            {saving ? (
              <LoaderCircle className="spin" size={15} />
            ) : (
              <Plus size={15} />
            )}
            {t("Create project")}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
