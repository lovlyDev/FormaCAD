import { useMemo, useState } from "react";
import { Button, Modal } from "../../components/ui";
import { number, t } from "../../i18n";
import type { Project } from "../../types";
import type { EdgeSelection } from "../viewer/edgeSelection";
import { ModelPreviewPanel } from "../model-preview/ModelPreviewPanel";
import { useModelPreview } from "../model-preview/useModelPreview";
import { parseTransformNumber } from "../transform/numericInput";
import { captureVariableFillet } from "./variableFilletCommand";
import { useFilletEndpoints } from "./useFilletEndpoints";
type Edit = NonNullable<ReturnType<typeof captureVariableFillet>>;

export function VariableFilletDialog({ project, selection, disabled, close, onApply, onConstant }: {
  project: Project; selection: readonly EdgeSelection[]; disabled: boolean; close: () => void;
  onApply: (edit: Edit) => void; onConstant: () => void;
}) {
  const [start, setStart] = useState("1");
  const [end, setEnd] = useState("3");
  const [enabled, setEnabled] = useState(true);
  const [refresh, setRefresh] = useState(0);
  const endpoints = useFilletEndpoints(project, selection, disabled);
  const draft = useMemo(() => {
    const a = parseTransformNumber(start, "length"), b = parseTransformNumber(end, "length");
    return a.status === "valid" && b.status === "valid" && endpoints?.start && endpoints.end
      ? captureVariableFillet(project, selection, a.value, b.value) : null;
  }, [project, selection, start, end, endpoints]);
  const preview = useModelPreview(project.id, draft?.previewSource ?? "", project.currentRevision,
    !!draft && !disabled && (enabled || refresh > 0), refresh);
  const point = (coordinates: number[]) => coordinates.map(value => number(value, 4)).join(" · ");
  return <Modal open wide onClose={close} title={t("Variable radius fillet")}
    description={t("Radii follow the exact BREP endpoints. Only one open edge is processed.")}>
    <form onSubmit={event => { event.preventDefault(); if (!disabled && draft && !preview.error) onApply(draft); }}>
      {selection.length !== 1 ? <p role="alert">{t("Select exactly one open edge for variable fillet.")}</p>
        : endpoints?.error ? <p role="alert">{endpoints.error}</p>
        : !endpoints?.start || !endpoints.end ? <p role="status">{t("Reading exact edge endpoints…")}</p>
        : <div>
          <p>{t("Start endpoint (CAD X, Y, Z), mm")}: <output>{point(endpoints.start)}</output></p>
          <p>{t("End endpoint (CAD X, Y, Z), mm")}: <output>{point(endpoints.end)}</output></p>
        </div>}
      <label className="field">{t("Start radius (mm)")}
        <input aria-label={t("Start radius (mm)")} inputMode="decimal" value={start} disabled={disabled} onChange={event => setStart(event.target.value)} />
      </label>
      <label className="field">{t("End radius (mm)")}
        <input aria-label={t("End radius (mm)")} inputMode="decimal" value={end} disabled={disabled} onChange={event => setEnd(event.target.value)} />
      </label>
      <p className="field-hint">{t("The rolling-ball radius varies linearly along the edge spine. Closed edges and expanded tangent chains are rejected.")}</p>
      <Button type="button" disabled={disabled} onClick={() => { setStart(end); setEnd(start); }}>{t("Swap endpoint radii")}</Button>
      <ModelPreviewPanel {...preview} enabled={enabled} disabled={disabled || !draft}
        onEnable={value => { setEnabled(value); setRefresh(0); }} onRefresh={() => setRefresh(value => value + 1)} />
      <div className="modal-actions">
        <Button type="button" disabled={disabled} onClick={onConstant}>{t("Constant radius or chamfer")}</Button>
        <Button type="button" onClick={close}>{t("Cancel")}</Button>
        <Button type="submit" className="primary" disabled={disabled || !draft || !!preview.error}>{t("Построить")}</Button>
      </div>
    </form>
  </Modal>;
}
