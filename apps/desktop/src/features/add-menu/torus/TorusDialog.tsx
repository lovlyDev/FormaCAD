import { useState } from "react";
import { Button, Modal } from "../../../components/ui";
import { t } from "../../../i18n";
import { parseTransformNumber } from "../../transform/numericInput";

export function TorusDialog({
  close,
  disabled,
  onApply,
}: {
  close: () => void;
  disabled: boolean;
  onApply: (majorRadiusMm: number, minorRadiusMm: number) => void;
}) {
  const [majorInput, setMajor] = useState("10");
  const [minorInput, setMinor] = useState("3");
  const major = parseTransformNumber(majorInput, "length");
  const minor = parseTransformNumber(minorInput, "length");
  const valid = major.status === "valid" && minor.status === "valid"
    && minor.value > 1e-7 && major.value - minor.value > 1e-7;
  return (
    <Modal open onClose={close} title={t("Add torus")}
      description={t("Create a solid ring around the Z axis at the model origin.")}>
      <form onSubmit={(event) => {
        event.preventDefault();
        if (valid && !disabled) onApply(major.value, minor.value);
      }}>
        <label className="field">
          {t("Major radius")} ({t("mm")})
          <input inputMode="decimal" value={majorInput} disabled={disabled}
            onChange={(event) => setMajor(event.target.value)} />
        </label>
        <label className="field">
          {t("Tube radius")} ({t("mm")})
          <input inputMode="decimal" value={minorInput} disabled={disabled}
            onChange={(event) => setMinor(event.target.value)} />
        </label>
        {!valid && <p role="status">{t("Enter two positive radii. The major radius must exceed the tube radius.")}</p>}
        <div className="modal-actions">
          <Button type="button" onClick={close}>{t("Cancel")}</Button>
          <Button type="submit" className="primary" disabled={disabled || !valid}>{t("Построить")}</Button>
        </div>
      </form>
    </Modal>
  );
}
