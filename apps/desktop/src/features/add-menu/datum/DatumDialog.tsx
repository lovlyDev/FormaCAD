import { useState } from "react";
import { Button, Modal, Select } from "../../../components/ui";
import { t } from "../../../i18n";
import { parseTransformNumber } from "../../transform/numericInput";
import { datumLabels, readDatumOperation, type DatumKind, type DatumOperation, type DatumSource } from "./datumModel";
import "./datumDialog.css";

const axes = ["X", "Y", "Z"] as const;

function millimetres(text: string) {
  const parsed = parseTransformNumber(text, "length");
  return parsed.status === "valid" ? parsed.value : NaN;
}

function vectorComponent(text: string) {
  const value = text.trim().replace(",", ".");
  return /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)$/.test(value) ? Number(value) : NaN;
}

function VectorFields({ name, values, onChange, disabled }: {
  name: string;
  values: string[];
  onChange: (values: string[]) => void;
  disabled: boolean;
}) {
  return (
    <fieldset>
      <legend>{name}</legend>
      <div className="datum-vector">
        {axes.map((axis, index) => (
          <label className="field" key={axis}>
            {axis}
            <input inputMode="decimal" value={values[index]} disabled={disabled}
              aria-label={t("{{field}} · {{axis}}", { field: name, axis })}
              onChange={(event) => onChange(values.map((value, i) => i === index ? event.target.value : value))} />
          </label>
        ))}
      </div>
    </fieldset>
  );
}

export type DatumSourceOption = { label: string; source: DatumSource };

export function DatumDialog({ kind, close, disabled, onApply, initial, error, sources = [] }: {
  kind: DatumKind;
  close: () => void;
  disabled: boolean;
  onApply: (operation: DatumOperation) => void;
  initial?: DatumOperation;
  error?: string;
  sources?: DatumSourceOption[];
}) {
  const anchor = initial?.type === "datumOnReference" ? initial.anchor : null;
  const [sourceIndex, setSourceIndex] = useState("");
  const [offset, setOffset] = useState(String(anchor?.offsetMm ?? 0));
  const [angle, setAngle] = useState(String(anchor?.angleDeg ?? 0));
  const [origin, setOrigin] = useState(initial && initial.type !== "datumOnReference"
    ? initial.originMm.map(String) : ["0", "0", "0"]);
  const [direction, setDirection] = useState(
    (initial?.type === "datumPlane" ? initial.normal : initial?.type === "datumAxis" ? initial.direction : [0, 0, 1]).map(String),
  );
  const [span, setSpan] = useState(String(
    initial?.type === "datumPlane" ? initial.sideMm : initial?.type === "datumAxis" ? initial.lengthMm
      : initial?.type === "datumOnReference" && initial.geometry.type === "plane" ? initial.geometry.sideMm
        : initial?.type === "datumOnReference" && initial.geometry.type === "axis" ? initial.geometry.lengthMm : 25,
  ));
  const originMm = origin.map(millimetres);
  const vector = direction.map(vectorComponent);
  const source = anchor?.source ?? (sourceIndex === "" ? undefined : sources[Number(sourceIndex)]?.source);
  const operation = readDatumOperation(source ? {
    type: "datumOnReference",
    geometry: kind === "datumPlane" ? { type: "plane", sideMm: millimetres(span) }
      : kind === "datumAxis" ? { type: "axis", lengthMm: millimetres(span) } : { type: "point" },
    anchor: { source, offsetMm: millimetres(offset), angleDeg: kind === "datumPoint" ? 0 : vectorComponent(angle) },
  } : kind === "datumPlane"
    ? { type: kind, originMm, normal: vector, sideMm: millimetres(span) }
    : kind === "datumAxis"
      ? { type: kind, originMm, direction: vector, lengthMm: millimetres(span) }
      : { type: kind, originMm });
  return (
    <Modal open onClose={close} title={t(datumLabels[kind])} className="datum-dialog"
      description={initial ? t("Adjust the last datum in the same Undo step.") : t("Add construction geometry in model coordinates.")}>
      <form onSubmit={(event) => {
        event.preventDefault();
        if (operation && !disabled) onApply(operation);
      }}>
        {!initial && sources.length > 0 && <label className="field">
          {t("Datum reference")}
          <Select value={sourceIndex} disabled={disabled} onChange={(event) => setSourceIndex(event.target.value)}>
            <option value="">{t("Model coordinates")}</option>
            {sources.map(({ label }, index) => <option key={index} value={index}>{label}</option>)}
          </Select>
        </label>}
        {source ? <>
          <label className="field">
            {t("Datum offset (mm)")}
            <input inputMode="decimal" value={offset} disabled={disabled} onChange={(event) => setOffset(event.target.value)} />
          </label>
          {kind !== "datumPoint" && <label className="field">
            {t("Datum angle (degrees)")}
            <input inputMode="decimal" value={angle} disabled={disabled} onChange={(event) => setAngle(event.target.value)} />
          </label>}
        </> : <VectorFields name={t("Datum origin (mm)")} values={origin} onChange={setOrigin} disabled={disabled} />}
        {kind !== "datumPoint" && <>
          {!source && <VectorFields name={kind === "datumPlane" ? t("Plane normal") : t("Axis direction")}
            values={direction} onChange={setDirection} disabled={disabled} />}
          <label className="field">
            {kind === "datumPlane" ? t("Plane side (mm)") : t("Axis length (mm)")}
            <input inputMode="decimal" value={span} disabled={disabled} onChange={(event) => setSpan(event.target.value)} />
          </label>
        </>}
        {!operation && <p role="status">{t("Check the datum coordinates, direction and size.")}</p>}
        {error && <p role="alert">{error}</p>}
        <div className="modal-actions">
          <Button type="button" onClick={close}>{t("Cancel")}</Button>
          <Button type="submit" className="primary" disabled={disabled || !operation}>{initial ? t("Apply") : t("Построить")}</Button>
        </div>
      </form>
    </Modal>
  );
}
