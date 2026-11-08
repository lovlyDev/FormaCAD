import { useMemo, useState } from "react";
import { Button, Modal } from "../../components/ui";
import { t } from "../../i18n";
import type { Project } from "../../types";
import { ModelPreviewPanel } from "../model-preview/ModelPreviewPanel";
import { useModelPreview } from "../model-preview/useModelPreview";
import { booleanLabels, booleanModes, captureBoolean, type BooleanMode } from "./booleanCommand";

type Edit = NonNullable<ReturnType<typeof captureBoolean>>;

export function BooleanDialog({ project, bodies, initialTarget, disabled, close, apply }: {
  project: Project;
  bodies: readonly { id: string; name: string }[];
  initialTarget: string;
  disabled: boolean;
  close: () => void;
  apply: (edit: Edit) => void;
}) {
  const [target, setTarget] = useState(initialTarget);
  const [tool, setTool] = useState("");
  const [mode, setMode] = useState<BooleanMode>("cut");
  const [autoPreview, setAutoPreview] = useState(true);
  const [refresh, setRefresh] = useState(0);
  const tools = bodies.filter((body) => body.id !== target);
  const toolId = tools.find((body) => body.id === tool)?.id ?? tools[0]?.id;
  const targetAvailable = bodies.some((body) => body.id === target);
  const draft = useMemo(() => !toolId || !targetAvailable ? null : captureBoolean(project, target, toolId, mode),
    [project, target, targetAvailable, toolId, mode]);
  const preview = useModelPreview(project.id, draft?.previewSource ?? "",
    project.currentRevision, !!draft && !disabled && (autoPreview || refresh > 0), refresh);
  return (
    <Modal open wide onClose={close} title={t("Boolean operation")}
      description={t("The target body changes. The tool body remains available separately.")}>
      <form onSubmit={(event) => {
        event.preventDefault();
        if (!disabled && draft && !preview.error) apply(draft);
      }}>
        <label className="field">
          {t("Target body")}
          <select value={target} onChange={(event) => setTarget(event.target.value)} disabled={disabled}>
            {bodies.map((body) => <option value={body.id} key={body.id}>{body.name || body.id}</option>)}
          </select>
        </label>
        <label className="field">
          {t("Tool body")}
          <select value={toolId ?? ""} onChange={(event) => setTool(event.target.value)} disabled={disabled}>
            {tools.map((body) => <option value={body.id} key={body.id}>{body.name || body.id}</option>)}
          </select>
        </label>
        <label className="field">
          {t("Boolean mode")}
          <select value={mode} onChange={(event) => {
            const value = booleanModes.find((mode) => mode === event.target.value);
            if (value) setMode(value);
          }} disabled={disabled}>
            {booleanModes.map((mode) => <option value={mode} key={mode}>{t(booleanLabels[mode])}</option>)}
          </select>
        </label>
        <ModelPreviewPanel {...preview} enabled={autoPreview} disabled={disabled || !draft}
          onEnable={(enabled) => { setAutoPreview(enabled); setRefresh(0); }}
          onRefresh={() => setRefresh((value) => value + 1)} />
        <div className="modal-actions">
          <Button type="button" onClick={close}>{t("Cancel")}</Button>
          <Button type="submit" className="primary" disabled={disabled || !draft || !!preview.error}>
            {t("Построить")}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
