import { useState } from "react";
import { Button, Modal } from "../../components/ui";
import { fixedNumber, t } from "../../i18n";
import { parseTransformNumber } from "../transform/numericInput";

export function FaceExtrusionDialog({ close, disabled, initialMm, onApply }: {
  close: () => void;
  disabled: boolean;
  initialMm?: number;
  onApply: (distanceMm: number) => void;
}) {
  const [distance, setDistance] = useState(() =>
    initialMm === undefined ? "" : fixedNumber(initialMm, 8).replace(/[.,]?0+$/, ""));
  const parsed = parseTransformNumber(distance, "length");
  const value = parsed.status === "valid" && Math.abs(parsed.value) > 1e-7
    ? parsed.value
    : null;
  return (
    <Modal open onClose={close} title={t("Extrude face")}
      description={t("Use a positive distance to add material or a negative distance to cut it.")}>
      <form onSubmit={(event) => {
        event.preventDefault();
        if (!disabled && value !== null) onApply(value);
      }}>
        <label className="field">
          {t("Extrusion distance (mm)")}
          <input autoFocus value={distance} inputMode="decimal" disabled={disabled}
            onChange={(event) => setDistance(event.target.value)} />
        </label>
        <div className="modal-actions">
          <Button type="button" onClick={close}>{t("Cancel")}</Button>
          <Button type="submit" className="primary" disabled={disabled || value === null}>
            {t("Построить")}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
