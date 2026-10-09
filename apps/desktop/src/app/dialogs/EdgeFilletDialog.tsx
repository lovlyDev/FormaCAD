import { useState } from "react";
import { t } from "../../i18n";
import { Button, Modal } from "../../components/ui";
import type { Project } from "../../types";

export function EdgeFilletDialog({ open, close, units, disabled, onApply }: {
  open: boolean;
  close: () => void;
  units: Project["units"];
  disabled: boolean;
  onApply: (radiusMm: number) => void;
}) {
  const [radius, setRadius] = useState("1");
  const factor = units === "inch" ? 25.4 : units === "cm" ? 10 : 1;
  const value = Number(radius) * factor;
  const valid = radius.trim() !== "" && Number.isFinite(value) && value > 0 && value <= 10000;
  return <Modal open={open} onClose={close} title={t("Fillet selected edge")}
    description={t("Set the radius for the highlighted edge. The model will be rebuilt as a new revision.")}>
    <label className="field">
      {t("Fillet radius")} ({t(units)})
      <input type="number" min={0} step="any" value={radius} disabled={disabled}
        onChange={(event) => setRadius(event.target.value)} />
    </label>
    <div className="modal-actions">
      <Button onClick={close}>{t("Cancel")}</Button>
      <Button className="primary" disabled={disabled || !valid} onClick={() => onApply(value)}>{t("Построить")}</Button>
    </div>
  </Modal>;
}
