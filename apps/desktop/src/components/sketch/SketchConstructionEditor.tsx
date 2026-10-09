import { PenLine,Plus,Trash2 } from "lucide-react";
import "./SketchTools.css";
import { Select } from "../ui";
import { useEditorPreference } from "../../features/model-editor/editorPreferences";
import { t } from "../../i18n";
import type { SketchOperation } from "../../lib/sketchDocument";
import { addConstructionLine, removeConstructionLine } from "../../lib/sketchConstruction";

export function SketchConstructionEditor({ operation, disabled, onChange, featureId="sketch" }: {
  featureId?: string;
  operation: SketchOperation;
  disabled: boolean;
  onChange: (operation: SketchOperation) => void;
}) {
  const [savedStart, setStart] = useEditorPreference(`sketch.${featureId}.construction.start`,operation.points[0]?.id ?? "");
  const [savedEnd, setEnd] = useEditorPreference(`sketch.${featureId}.construction.end`,operation.points[2]?.id ?? "");
  const start=operation.points.some(point=>point.id===savedStart)?savedStart:operation.points[0]?.id??"";
  const end=operation.points.some(point=>point.id===savedEnd)?savedEnd:operation.points[1]?.id??"";
  const canAdd = start !== end && operation.lines.length < 64
    && operation.points.some((point) => point.id === start)
    && operation.points.some((point) => point.id === end);
  return <div className="sketch-editor-construction-tools sketch-tool-card">
    <h4><PenLine size={15}/>{t("Construction geometry")}</h4>
    <p className="sketch-editor-hint">{t("Construction lines constrain the sketch but are excluded from extrusion.")}</p>
    <div className="sketch-editor-construction-add">
      <label>{t("Construction start point")}
        <Select aria-label={t("Construction start point")} value={start} disabled={disabled} onChange={(event) => setStart(event.target.value)}>
          {operation.points.map((point) => <option key={point.id} value={point.id}>{point.id}</option>)}
        </Select>
      </label>
      <label>{t("Construction end point")}
        <Select aria-label={t("Construction end point")} value={end} disabled={disabled} onChange={(event) => setEnd(event.target.value)}>
          {operation.points.map((point) => <option key={point.id} value={point.id}>{point.id}</option>)}
        </Select>
      </label>
      <button type="button" disabled={disabled || !canAdd} onClick={() => {
        const next = addConstructionLine(operation, start, end);
        if (next) onChange(next);
      }}><Plus size={14}/>{t("Add construction line")}</button>
    </div>
    {operation.lines.filter((line) => line.construction).map((line) => <div key={line.id} className="sketch-editor-construction-row">
      <span>{line.id} · {line.startPointId} → {line.endPointId}</span>
      <button type="button" disabled={disabled}
        aria-label={t("Remove construction line {{value0}}", { value0: line.id })}
        onClick={() => onChange(removeConstructionLine(operation, line.id))}><Trash2 size={14}/>{t("Remove construction line")}</button>
    </div>)}
  </div>;
}
