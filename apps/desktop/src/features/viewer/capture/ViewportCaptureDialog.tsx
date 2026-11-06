import { useState } from "react";
import { Button, Checkbox, Modal } from "../../../components/ui";
import { errorText, t } from "../../../i18n";
import type { ViewportPngOptions } from "./viewportPng";

export function ViewportCaptureDialog({ close, capture }: {
  close: () => void;
  capture: (options: ViewportPngOptions) => void;
}) {
  const [scale, setScale] = useState<ViewportPngOptions["scale"]>(1);
  const [transparent, setTransparent] = useState(false);
  const [error, setError] = useState("");
  return (
    <Modal open onClose={close} title={t("Viewport snapshot")}
      description={t("Capture the current 3D view as a PNG attachment.")}>
      <form onSubmit={(event) => {
        event.preventDefault();
        try {
          capture({ scale, transparent });
          close();
        } catch (cause) {
          setError(errorText(cause));
        }
      }}>
        <label className="field">{t("PNG scale")}
          <select aria-label={t("PNG scale")} value={scale} onChange={(event) => {
            const value = event.target.value;
            setScale(value === "3" ? 3 : value === "2" ? 2 : 1);
            setError("");
          }}>
            <option value={1}>1×</option>
            <option value={2}>2×</option>
            <option value={3}>3×</option>
          </select>
        </label>
        <Checkbox checked={transparent} onChange={setTransparent}>{t("Transparent background")}</Checkbox>
        {error && <p role="alert">{error}</p>}
        <div className="modal-actions">
          <Button type="button" onClick={close}>{t("Cancel")}</Button>
          <Button type="submit" className="primary">{t("Attach viewport to chat")}</Button>
        </div>
      </form>
    </Modal>
  );
}
