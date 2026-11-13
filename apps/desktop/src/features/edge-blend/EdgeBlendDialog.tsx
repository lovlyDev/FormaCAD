import { useMemo, useState } from "react";
import { Button, Modal } from "../../components/ui";
import { fixedNumber, t } from "../../i18n";
import type { Project } from "../../types";
import { ModelPreviewPanel } from "../model-preview/ModelPreviewPanel";
import { useModelPreview } from "../model-preview/useModelPreview";
import { parseTransformNumber } from "../transform/numericInput";
import type { EdgeSelection } from "../viewer/edgeSelection";
import { captureEdgeBlend, type EdgeBlendMode } from "./edgeBlendCommand";
import { VariableFilletDialog } from "../variable-fillet/VariableFilletDialog";

type Blend = NonNullable<ReturnType<typeof captureEdgeBlend>>;

export function EdgeBlendDialog({ project, selection, disabled, close, onApply }: {
  project: Project;
  selection: readonly EdgeSelection[];
  disabled: boolean;
  close: () => void;
  onApply: (blend: Blend) => void;
}) {
  const [mode, setMode] = useState<EdgeBlendMode>("fillet");
  const [variable, setVariable] = useState(false);
  const [size, setSize] = useState("1");
  const [autoPreview, setAutoPreview] = useState(true);
  const [refresh, setRefresh] = useState(0);
  const parsed = parseTransformNumber(size, "length");
  const mm = parsed.status === "valid" && parsed.value > 0 ? parsed.value : null;
  const draft = useMemo(() => mm === null ? null : captureEdgeBlend(project, selection, mode, mm),
    [project, selection, mode, mm]);
  const preview = useModelPreview(project.id, draft?.previewSource ?? "",
    project.currentRevision, !!draft && !disabled && (autoPreview || refresh > 0), refresh);
  const label = mode === "fillet" ? t("Fillet radius") : t("Расстояние");
  if (variable) return <VariableFilletDialog project={project} selection={selection} disabled={disabled}
    close={close} onApply={onApply} onConstant={() => setVariable(false)} />;
  return (
    <Modal open wide onClose={close} title={t("Blend selected edges")}
      description={t("Review exact geometry before saving a new revision.")}>
      <form onSubmit={(event) => {
        event.preventDefault();
        if (!disabled && draft && !preview.error) onApply(draft);
      }}>
        <p className="field-hint">{t("Selected edges: {{value0}}", { value0: selection.length })}</p>
        <label className="field">
          {t("Edge treatment")}
          <select aria-label={t("Edge treatment")} value={mode} disabled={disabled}
            onChange={(event) => { if (event.target.value === "variable") setVariable(true);
              else setMode(event.target.value === "chamfer" ? "chamfer" : "fillet"); }}>
            <option value="fillet">{t("fillet")}</option>
            <option value="chamfer">{t("chamfer")}</option>
            <option value="variable">{t("Variable radius fillet")}</option>
          </select>
        </label>
        <label className="field">
          {label} ({t("mm")})
          <input value={size} inputMode="decimal" disabled={disabled}
            onChange={(event) => setSize(event.target.value)} />
        </label>
        <input type="range" min={0.05} max={Math.max(10, Math.ceil(mm ?? 0))} step={0.05}
          value={Math.max(0.05, mm ?? 0.05)} disabled={disabled}
          aria-label={t("Adjust edge treatment size")}
          onChange={(event) => setSize(fixedNumber(event.target.valueAsNumber, 2))} />
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
