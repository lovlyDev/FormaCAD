import { Button, Modal } from "../../components/ui";
import { t } from "../../i18n";
import type { IgesExportReceipt } from "./igesApi";
import { IgesMeasurements } from "./IgesMeasurements";
export function IgesExportReportDialog({ receipt, close }: { receipt: IgesExportReceipt; close: () => void }) {
  return <Modal open wide onClose={close} title={t("IGES export verified")}
    description={t("The exported file was read back by OCCT and its measurements were checked.")}>
    <p className="iges-exchange-path">{receipt.path}</p>
    <IgesMeasurements report={receipt.report} />
    <p className="field-hint">{t("IGES contains exact BREP solids in millimeters. Revision history and appearance are not part of this exchange format.")}</p>
    <div className="modal-actions"><Button onClick={close}>{t("Close")}</Button></div>
  </Modal>;
}
