import { ChevronDown, Rotate3d, FlipHorizontal2 } from "lucide-react";
import { PersistentDetails } from "../model-editor/editorPreferences";
import { t } from "../../i18n";
import type { TransformOperation } from "./transformOperation";
import "./TransformFeatureEditor.css";

export function TransformFeatureEditor({ featureId, operation, disabled, onChange }: {
  featureId: string; operation: TransformOperation; disabled: boolean; onChange: (operation: TransformOperation) => void;
}) {
  const rotate = operation.type !== "mirror";
  const Icon = rotate ? Rotate3d : FlipHorizontal2;
  const origin = rotate ? operation.axisOriginMm : operation.planeOriginMm;
  const direction = rotate ? operation.axisDirection : operation.planeNormal;
  const vector = (label: string, values: [number, number, number], originField: boolean) => <fieldset disabled={disabled}>
    <legend>{label}</legend><div className="transform-vector">{values.map((value, index) => <label key={index}>
      <span>{["X", "Y", "Z"][index]}</span><input type="number" step="any" value={value} min={originField ? -10000 : -1e6} max={originField ? 10000 : 1e6}
        aria-label={`${label} ${["X", "Y", "Z"][index]}`} onChange={event => {
          const n = event.target.valueAsNumber;
          if (!Number.isFinite(n) || Math.abs(n) > (originField ? 10000 : 1e6)) return;
          const next: [number, number, number] = [...values]; next[index] = n;
          if (rotate) onChange({ ...operation, [originField ? "axisOriginMm" : "axisDirection"]: next });
          else onChange({ ...operation, [originField ? "planeOriginMm" : "planeNormal"]: next });
        }} /></label>)}</div>
  </fieldset>;
  return <PersistentDetails stateId={`transform.${featureId}`} className="transform-editor">
    <summary><Icon size={15} /><span>{operation.type === "revolve" ? t("Edit revolution") : t("Edit transform")}</span><ChevronDown size={15} className="cad-section-chevron" /></summary>
    <div className="transform-editor-content"><p className="field-hint">{operation.type === "revolve" ? t("A closed sketch revolves around an axis in its plane. The profile must stay on one side of the axis; the angle must be nonzero.") : t("Transforms preserve the source solid. Changes become a revision only after model validation and saving.")}</p>
      {vector(rotate ? t("Axis origin in mm") : t("Mirror plane origin in mm"), origin, true)}
      {vector(rotate ? t("Axis direction (unitless)") : t("Mirror plane normal (unitless)"), direction, false)}
      {rotate && <label className="transform-angle">{t("Rotation angle in degrees")}<input type="number" step="any" min={-360} max={360} value={operation.angleDeg} disabled={disabled} onChange={event => {
        const angle = event.target.valueAsNumber; if (Number.isFinite(angle) && Math.abs(angle) <= 360) onChange({ ...operation, angleDeg: angle });
      }} /></label>}
    </div>
  </PersistentDetails>;
}
