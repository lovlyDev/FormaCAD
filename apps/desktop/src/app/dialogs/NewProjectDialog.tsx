import { useState } from "react";
import { LoaderCircle, Plus, FileBox } from "lucide-react";
import { t } from "../../i18n";
import { Button, Modal, Select } from "../../components/ui";
import { newProject } from "../../stores/workspace";
import type { Agent, Project } from "../../types";
export function NewProjectDialog({
  open,
  close,
  onCreate,
  importNames = [],
}: {
  importNames?: string[];
  open: boolean;
  close: () => void;
  onCreate: (p: Project) => Promise<void>;
}) {
  const [name, setName] = useState(t("Untitled part"));
  const [units, setUnits] = useState<Project["units"]>("mm");
  const [agent, setAgent] = useState<Agent>("codex");
  const [saving, setSaving] = useState(false);
  return (
    <Modal
      open={open}
      onClose={close}
      title={importNames.length ? t("Create a project from selected files") : t("Make room for an idea")}
      description={importNames.length ? t("Selected models will be imported. STEP files need local conversion before they appear in the scene.") : t("Start with an empty workspace.")}
    >
      {importNames.length > 0 && <ul className="import-file-list">{importNames.map(name => <li key={name}><FileBox size={15}/><span>{name}</span></li>)}</ul>}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (!name.trim() || saving) return;
          setSaving(true);
          void onCreate(newProject(name, "blank", units, agent)).finally(() =>
            setSaving(false),
          );
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
        <div className="modal-actions">
          <Button type="button" onClick={close}>
            {t("Cancel")}
          </Button>
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
