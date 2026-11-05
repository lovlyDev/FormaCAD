import { useState } from "react";
import { Button, Modal } from "../../components/ui";
import { fixedNumber, t } from "../../i18n";
import { parseTransformNumber } from "../transform/numericInput";

export function DuplicateBodyDialog({ sourceName, spacingMm, disabled, close, onApply }: {
  sourceName: string;
  spacingMm: number;
  disabled: boolean;
  close: () => void;
  onApply: (name: string, offsetMm: [number, number, number]) => void;
}) {
  const [name, setName] = useState(() =>
    Array.from(t("{{value0}} copy", { value0: sourceName })).slice(0, 120).join(""));
  const [offset, setOffset] = useState(() => [fixedNumber(spacingMm), "0", "0"]);
  const [x, y, z] = offset.map((value) => parseTransformNumber(value, "length"));
  const valid = x.status === "valid" && y.status === "valid" && z.status === "valid" &&
    name.trim().length > 0 && Array.from(name.trim()).length <= 120 && !/\p{Cc}/u.test(name);
  return (
    <Modal open onClose={close} title={t("Duplicate body")}
      description={t("Make an independent copy of this body's modeling history and parameters.")}>
      <form onSubmit={(event) => {
        event.preventDefault();
        if (valid && !disabled) onApply(name, [x.value, y.value, z.value]);
      }}>
        <label className="field">{t("Copy name")}
          <input value={name} disabled={disabled} onChange={(event) => setName(event.target.value)} />
        </label>
        {(["X", "Y", "Z"] as const).map((axis, index) => (
          <label key={axis} className="field">{t("Copy offset {{axis}} (mm)", { axis })}
            <input inputMode="decimal" value={offset[index]} disabled={disabled}
              onChange={(event) => {
                const value = event.currentTarget.value;
                setOffset((current) => current.map((item, slot) => slot === index ? value : item));
              }} />
          </label>
        ))}
        <div className="modal-actions">
          <Button type="button" onClick={close}>{t("Cancel")}</Button>
          <Button type="submit" className="primary" disabled={disabled || !valid}>{t("Duplicate body")}</Button>
        </div>
      </form>
    </Modal>
  );
}
