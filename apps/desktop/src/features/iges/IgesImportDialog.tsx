import { useEffect, useState, lazy, Suspense } from "react";
import type { Group } from "three";
import { Button, Checkbox, Modal } from "../../components/ui";
import { rawError, t } from "../../i18n";
import { loadModel } from "../../lib/files";
import { disposeModel } from "../../lib/model";
import type { Project } from "../../types";
import { readIgesInput, previewIges, type IgesInput } from "./igesApi";
import type { IgesReport } from "./igesSchema";
import { captureIgesImport } from "./igesImportCommand";
import { IgesMeasurements } from "./IgesMeasurements";
import { igesErrorMessage } from "./igesError";
const PreviewCanvas = lazy(() => import("../model-preview/PreviewCanvas").then(module => ({ default: module.PreviewCanvas })));
type Edit = NonNullable<ReturnType<typeof captureIgesImport>>;
export function IgesImportDialog({ project, file, disabled, close, apply }: {
  project: Project; file: File; disabled: boolean; close: () => void; apply: (edit: Edit) => void;
}) {
  const [ready, setReady] = useState<{ input: IgesInput; report: IgesReport; object: Group } | null>(null);
  const [error, setError] = useState<unknown>();
  const [confirmed, setConfirmed] = useState(!project.currentRevision);
  const [fit, setFit] = useState(0);
  useEffect(() => {
    let canceled = false, object: Group | undefined;
    void (async () => {
      const input = await readIgesInput(project, file), preview = await previewIges(input);
      const loaded = await loadModel({ name: "preview.glb", kind: "model", data: preview.previewData, size: preview.report.previewSize! });
      object = loaded;
      if (canceled) disposeModel(loaded); else setReady({ input, report: preview.report, object: loaded });
    })().catch(cause => { if (!canceled) setError(cause); });
    return () => { canceled = true; if (object) disposeModel(object); };
  }, [project, file]);
  const edit = ready ? captureIgesImport(project, ready.input, ready.report.sourceSha256) : null;
  return <Modal open wide onClose={close} title={t("Import IGES model")}
    description={t("Review the exact solid conversion. Import creates a new revision and retains earlier history.")}>
    <p className="field-hint">{file.name}</p>
    {error !== undefined ? <div><p role="alert">{igesErrorMessage(error)}</p><details><summary>{t("Technical details")}</summary><pre>{rawError(error)}</pre></details></div> : !ready ? <p role="status">{t("Reading IGES geometry…")}</p>
      : <>
        <IgesMeasurements report={ready.report} />
        <section className="model-preview-panel" aria-label={t("3D draft preview")}>
          <Button type="button" onClick={() => setFit(v => v + 1)}>{t("Fit preview to view")}</Button>
          <Suspense fallback={<p>{t("Loading preview…")}</p>}><PreviewCanvas object={ready.object} fit={fit} /></Suspense>
        </section>
        <p role="status">{t("IGES round-trip verified. All closed solids are preserved.")}</p>
      </>}
    <p className="field-hint">{t("IGES is normalized to millimeters. Surface-only files are rejected; no solid is guessed or chosen automatically.")}</p>
    {project.currentRevision && <Checkbox checked={confirmed} disabled={disabled} onChange={setConfirmed}>
      {t("Replace the current model in a new revision. Earlier revisions remain available.")}
    </Checkbox>}
    <div className="modal-actions">
      <Button type="button" onClick={close}>{t("Cancel")}</Button>
      <Button className="primary" disabled={disabled || !edit || !confirmed || error !== undefined} onClick={() => { if (edit) apply(edit); }}>{t("Import IGES")}</Button>
    </div>
  </Modal>;
}
