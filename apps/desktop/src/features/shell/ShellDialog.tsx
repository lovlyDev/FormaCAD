import { useState } from "react";
import { Button, Checkbox, Modal } from "../../components/ui";
import { t } from "../../i18n";
import { parseTransformNumber } from "../transform/numericInput";
import type { FaceTopologyReference } from "../viewer/topology/topologyReference";

export function ShellDialog({ primary, faces, disabled, close, onApply }: {
  primary: FaceTopologyReference;
  faces: FaceTopologyReference[];
  disabled: boolean;
  close: () => void;
  onApply: (openings: FaceTopologyReference[], offsetMm: number) => void;
}) {
  const [roles, setRoles] = useState<string[]>([primary.role]);
  const [thickness, setThickness] = useState("1");
  const [outward, setOutward] = useState(false);
  const parsed = parseTransformNumber(thickness, "length");
  const mm = parsed.status === "valid" && parsed.value > 1e-7 ? parsed.value : null;
  const openings = faces.filter((face) => roles.includes(face.role));
  const valid = mm !== null && openings.length > 0 && openings.length < faces.length;
  return (
    <Modal open onClose={close} title={t("Shell")}
      description={t("Choose which faces to remove and how thick to make the walls.")}>
      <form onSubmit={(event) => {
        event.preventDefault();
        if (!disabled && valid) onApply(openings, outward ? mm : -mm);
      }}>
        <fieldset disabled={disabled}>
          <legend>{t("Opening faces")}</legend>
          {faces.map((face) => (
            <Checkbox key={face.role} checked={roles.includes(face.role)} onChange={(checked) =>
              setRoles((current) => checked ? [...current, face.role] : current.filter((role) => role !== face.role))}>
              {face.role}
            </Checkbox>
          ))}
        </fieldset>
        <label className="field">
          {t("Wall thickness")} ({t("mm")})
          <input value={thickness} inputMode="decimal" disabled={disabled}
            onChange={(event) => setThickness(event.target.value)} />
        </label>
        <Checkbox checked={outward} disabled={disabled} onChange={setOutward}>
          {t("Build walls outward")}
        </Checkbox>
        <div className="modal-actions">
          <Button type="button" onClick={close}>{t("Cancel")}</Button>
          <Button type="submit" className="primary" disabled={disabled || !valid}>{t("Построить")}</Button>
        </div>
      </form>
    </Modal>
  );
}
