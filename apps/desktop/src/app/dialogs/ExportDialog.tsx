import { Download } from "lucide-react";
import { t } from "../../i18n";
import { Button, Modal, Select } from "../../components/ui";
import { native } from "../../lib/api";
import { usePersistentState } from "../../lib/persistence";
import type { ExportBody } from "../../features/viewer/exportBodies";
export function ExportDialog({
  open,
  close,
  onExport,
  hasModel,
  bodies,
}: {
  open: boolean;
  close: () => void;
  onExport: (format: string, bodyId: string | null) => Promise<void>;
  hasModel: boolean;
  bodies: ExportBody[];
}) {
  const [format, setFormat] = usePersistentState("forma.ui.exportFormat", "stl");
  const [bodyId, setBodyId] = usePersistentState("forma.ui.exportBody", "");
  const selectedBodyId = bodies.some((body) => body.id === bodyId) ? bodyId : "";
  return (
    <Modal
      open={open}
      onClose={close}
      title={t("Ready for the next step")}
      description={t("Choose an export format.")}
    >
      <label>
        {t("File format")}
        <Select value={format} onChange={(e) => setFormat(e.target.value)}>
          <option value="stl">{t("STL — 3D printing mesh")}</option>
          <option value="3mf">{t("3MF — 3D printing assembly")}</option>
          <option value="glb">{t("GLB — portable 3D model")}</option>
          <option value="obj">{t("OBJ — polygon mesh")}</option>
          <option value="step" disabled={!native}>
            {t("STEP — solid CAD (desktop)")}
          </option>
        </Select>
      </label>
      {bodies.length > 1 && (
        <label>
          {t("Export scope")}
          <Select value={selectedBodyId} onChange={(event) => setBodyId(event.target.value)}>
            <option value="">{t("Entire model")}</option>
            {bodies.map((body) => (
              <option key={body.id} value={body.id}>{body.name}</option>
            ))}
          </Select>
        </label>
      )}
      <div className="export-spec">
        <span>
          {t("Units")}
          <strong>{t("Millimeters")}</strong>
        </span>
        <span>
          {t("Geometry")}
          <strong>
            {format === "step" ? t("Exact solid") : t("Current mesh")}
          </strong>
        </span>
      </div>
      <p className="field-hint">
        {t(
          "STL and OBJ are unitless formats; exported coordinates are in millimeters. Export does not alter your project.",
        )}
      </p>
      <div className="modal-actions">
        <Button onClick={close}>{t("Cancel")}</Button>
        <Button
          className="primary"
          disabled={!hasModel}
          onClick={() => void onExport(format, selectedBodyId || null)}
        >
          <Download size={15} />
          {t("Export")} {format.toUpperCase()}
        </Button>
      </div>
    </Modal>
  );
}
