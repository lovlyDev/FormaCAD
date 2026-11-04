import { useState } from "react";
import { Button, Modal } from "../../../components/ui";
import { t } from "../../../i18n";
import { parseTransformNumber } from "../../transform/numericInput";
import { datumLabels, readDatumOperation, type DatumKind, type DatumOperation } from "./datumModel";
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

export function DatumDialog({ kind, close, disabled, onApply }: {
  kind: DatumKind;
  close: () => void;
  disabled: boolean;
  onApply: (operation: DatumOperation) => void;
}) {
  const [origin, setOrigin] = useState(["0", "0", "0"]);
  const [direction, setDirection] = useState(["0", "0", "1"]);
  const [span, setSpan] = useState("25");
  const originMm = origin.map(millimetres);
  const vector = direction.map(vectorComponent);
  const operation = readDatumOperation(kind === "datumPlane"
    ? { type: kind, originMm, normal: vector, sideMm: millimetres(span) }
    : kind === "datumAxis"
      ? { type: kind, originMm, direction: vector, lengthMm: millimetres(span) }
      : { type: kind, originMm });
  return (
    <Modal open onClose={close} title={t(datumLabels[kind])} className="datum-dialog"
      description={t("Add construction geometry in model coordinates.")}>
      <form onSubmit={(event) => {
        event.preventDefault();
        if (operation && !disabled) onApply(operation);
      }}>
        <VectorFields name={t("Datum origin (mm)")} values={origin} onChange={setOrigin} disabled={disabled} />
        {kind !== "datumPoint" && <>
          <VectorFields name={kind === "datumPlane" ? t("Plane normal") : t("Axis direction")}
            values={direction} onChange={setDirection} disabled={disabled} />
          <label className="field">
            {kind === "datumPlane" ? t("Plane side (mm)") : t("Axis length (mm)")}
            <input inputMode="decimal" value={span} disabled={disabled} onChange={(event) => setSpan(event.target.value)} />
          </label>
        </>}
        {!operation && <p role="status">{t("Check the datum coordinates, direction and size.")}</p>}
        <div className="modal-actions">
          <Button type="button" onClick={close}>{t("Cancel")}</Button>
          <Button type="submit" className="primary" disabled={disabled || !operation}>{t("Построить")}</Button>
        </div>
      </form>
    </Modal>
  );
}
