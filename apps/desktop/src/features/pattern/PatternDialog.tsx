import { useState } from "react";
import { Button, Checkbox, Modal } from "../../components/ui";
import { t } from "../../i18n";
import { parseTransformNumber, type TransformQuantity } from "../transform/numericInput";
import type { PatternLayout, PatternVector } from "./patternCommand";

const axes = ["X", "Y", "Z"] as const;
type Axis = typeof axes[number];
type TextVector = [string, string, string];

function numberValue(text: string, quantity: TransformQuantity) {
  const parsed = parseTransformNumber(text, quantity);
  return parsed.status === "valid" ? parsed.value : null;
}

function countValue(text: string, minimum: number) {
  const count = /^\d+$/.test(text.trim()) ? Number(text) : 0;
  return count >= minimum && count <= 128 ? count : null;
}

function along(axis: Axis, distance: number): PatternVector {
  return [axis === "X" ? distance : 0, axis === "Y" ? distance : 0, axis === "Z" ? distance : 0];
}

function AxisField({ label, axis, change }: { label: string; axis: Axis; change: (axis: Axis) => void }) {
  return <label className="field">{label}
    <select aria-label={label} value={axis} onChange={(event) => change(event.target.value as Axis)}>
      {axes.map((item) => <option key={item} value={item}>{item}</option>)}
    </select>
  </label>;
}

export function PatternDialog({ canRepeatHole, disabled, close, onApply }: {
  canRepeatHole: boolean;
  disabled: boolean;
  close: () => void;
  onApply: (repeatHole: boolean, layout: PatternLayout) => void;
}) {
  const [circular, setCircular] = useState(false);
  const [repeatHole, setRepeatHole] = useState(false);
  const [count, setCount] = useState("3");
  const [rows, setRows] = useState("1");
  const [firstStep, setFirstStep] = useState("50");
  const [secondStep, setSecondStep] = useState("50");
  const [angle, setAngle] = useState("90");
  const [firstAxis, setFirstAxis] = useState<Axis>("X");
  const [secondAxis, setSecondAxis] = useState<Axis>("Y");
  const [rotationAxis, setRotationAxis] = useState<Axis>("Z");
  const [origin, setOrigin] = useState<TextVector>(["0", "0", "0"]);

  const instances = countValue(count, 2);
  const rowCount = countValue(rows, 1);
  const u = numberValue(firstStep, "length");
  const v = rowCount === 1 ? 0 : numberValue(secondStep, "length");
  const stepDeg = numberValue(angle, "angle");
  const [x, y, z] = origin.map((value) => numberValue(value, "length"));
  let layout: PatternLayout | null = null;
  if (circular && instances && stepDeg !== null && x !== null && y !== null && z !== null &&
      Math.abs(stepDeg) > 1e-7 && Math.abs(stepDeg) * (instances - 1) < 360 - 1e-7) {
    layout = { type: "patternCircular", layout: { count: instances, axisOriginMm: [x, y, z],
      axisDirection: along(rotationAxis, 1), stepDeg } };
  }
  if (!circular && instances && rowCount && instances * rowCount <= 128 && u !== null && v !== null &&
      Math.abs(u) > 1e-7 && Math.abs(u) * (instances - 1) <= 10000 &&
      (rowCount === 1 || (firstAxis !== secondAxis && Math.abs(v) > 1e-7 &&
        Math.abs(v) * (rowCount - 1) <= 10000))) {
    layout = { type: "patternLinear", layout: { countU: instances, countV: rowCount,
      stepUMm: along(firstAxis, u), stepVMm: along(secondAxis, v) } };
  }
  return (
    <Modal open wide onClose={close} title={t("Pattern")}
      description={t("Repeat a body or its active hole at regular intervals.")}>
      <form onSubmit={(event) => {
        event.preventDefault();
        if (layout && !disabled) onApply(canRepeatHole && repeatHole, layout);
      }}>
        <fieldset disabled={disabled}>
          <label className="field">{t("Pattern layout")}
            <select aria-label={t("Pattern layout")} value={circular ? "circular" : "linear"}
              onChange={(event) => setCircular(event.target.value === "circular")}>
              <option value="linear">{t("patternLinear")}</option>
              <option value="circular">{t("patternCircular")}</option>
            </select>
          </label>
          {canRepeatHole && <Checkbox checked={repeatHole} onChange={setRepeatHole}>{t("Repeat the hole only")}</Checkbox>}
          <label className="field">{t("Instance count")}
            <input inputMode="numeric" value={count} onChange={(event) => setCount(event.target.value)} />
          </label>
          {circular ? <>
            <AxisField label={t("Rotation axis")} axis={rotationAxis} change={setRotationAxis} />
            <label className="field">{t("Angular step (degrees)")}
              <input inputMode="decimal" value={angle} onChange={(event) => setAngle(event.target.value)} />
            </label>
            {axes.map((axis, index) => (
              <label className="field" key={axis}>{t("Axis origin {{axis}} (mm)", { axis })}
                <input inputMode="decimal" value={origin[index]} onChange={(event) =>
                  setOrigin((current) => {
                    const next: TextVector = [...current];
                    next[index] = event.target.value;
                    return next;
                  })} />
              </label>
            ))}
          </> : <>
            <AxisField label={t("First pattern axis")} axis={firstAxis} change={setFirstAxis} />
            <label className="field">{t("First spacing (mm)")}
              <input inputMode="decimal" value={firstStep} onChange={(event) => setFirstStep(event.target.value)} />
            </label>
            <label className="field">{t("Row count")}
              <input inputMode="numeric" value={rows} onChange={(event) => setRows(event.target.value)} />
            </label>
            {rowCount !== null && rowCount > 1 && <>
              <AxisField label={t("Second pattern axis")} axis={secondAxis} change={setSecondAxis} />
              <label className="field">{t("Second spacing (mm)")}
                <input inputMode="decimal" value={secondStep} onChange={(event) => setSecondStep(event.target.value)} />
              </label>
            </>}
          </>}
        </fieldset>
        <div className="modal-actions">
          <Button type="button" onClick={close}>{t("Cancel")}</Button>
          <Button type="submit" className="primary" disabled={disabled || !layout}>{t("Построить")}</Button>
        </div>
      </form>
    </Modal>
  );
}
