import { lazy, Suspense, useState, useMemo } from "react";
import { Check, X, Box, Maximize, LoaderCircle, RotateCw, AlertCircle, Layers, Sparkles } from "lucide-react";
import { Button } from "../../../components/ui";
import { t, number, useLocale } from "../../../i18n";
import { useWorkspace } from "../../../stores/workspace";
import type { Project } from "../../../types";
import { readTypedCadDocument } from "../../../lib/typedCadDocument";
import { candidateSourceStatus } from "../candidateSource";
import { useModelPreview } from "../../model-preview/useModelPreview";
import { previewError } from "../../model-preview/previewError";
import { CandidateChanges } from "./CandidateChanges";
import { CandidatePlan } from "./plan/CandidatePlan";
import { checkReviewPlan, type ReviewPlan } from "./plan/reviewPlan";
import { useSavedBaseline } from "./useSavedBaseline";
import "../../model-preview/ModelPreviewPanel.css";
import "./CandidateReview.css";

const PreviewCanvas = lazy(() => import("../../model-preview/PreviewCanvas").then(module => ({ default: module.PreviewCanvas })));
export type CandidateReviewData = { base: Project; source: string; reviewPlan?: ReviewPlan | null };

export function CandidateReview({ review, onDecision }: { review: CandidateReviewData; onDecision: (allow: boolean) => void }) {
  useLocale();
  const current = useWorkspace(state => state.project);
  const validBase = !!current && candidateSourceStatus(current, review.base) === "valid";
  const [refresh, setRefresh] = useState(0), [fit, setFit] = useState(0), [view, setView] = useState<"candidate" | "current">("candidate");
  const preview = useModelPreview(review.base.id, review.source, review.base.currentRevision, validBase, refresh);
  const baseline = useSavedBaseline(review.base);
  const geometryReady = validBase && preview.available && !preview.pending && !preview.waiting && !!preview.preview && !preview.error;
  const object = view === "candidate" ? preview.preview?.object : baseline.object;
  const finding = preview.error ? previewError(preview.error) : null;
  const document = useMemo(() => readTypedCadDocument(review.source), [review.source]);
  const checks = review.reviewPlan ? checkReviewPlan(review.reviewPlan, document, preview.preview?.metrics) : [];
  const ready = geometryReady && checks.every(Boolean);
  return <>
    <div className="model-editor-toolbar"><span><Box size={15} />{t("CAD IR v2")}</span><span className="model-editor-draft">{t("AI candidate")}</span>
      {document && <span className="model-editor-counts"><Layers size={13} />{t("{{value0}} operations · {{value1}} bodies", { value0: number(document.features.length, 0), value1: number(document.bodies.length, 0) })}</span>}</div>
    <div className="model-editor-scroll candidate-review-scroll">
      <p className="candidate-review-note">{t("Review this AI candidate before accepting. The saved model and history stay unchanged until acceptance.")}</p>
      {!validBase && <p className="candidate-review-error" role="alert"><AlertCircle size={16} />{t("The source project or revision changed. Reject this candidate and request a new plan.")}</p>}
      <div className="model-editor-layout candidate-review-layout">
        <div><CandidatePlan plan={review.reviewPlan} results={checks} checked={geometryReady} /><CandidateChanges base={review.base} source={review.source} /></div>
        <section className="model-preview-panel candidate-geometry" aria-label={t("AI candidate geometry")}>
          <header><Box size={18} /><strong>{t("AI candidate geometry")}</strong><span className="preview-readonly">{t("Read-only")}</span></header>
          <div className="candidate-view-actions">
            <div role="group" aria-label={t("Compare model views")}>
              <Button aria-pressed={view === "candidate"} onClick={() => setView("candidate")}><Sparkles size={14} />{t("AI candidate")}</Button>
              <Button aria-pressed={view === "current"} onClick={() => setView("current")}><Box size={14} />{t("Current model")}</Button>
            </div>
            <Button disabled={!object} aria-label={t("Fit preview to view")} onClick={() => setFit(value => value + 1)}><Maximize size={16} /></Button>
          </div>
          <div className="model-preview-stage candidate-review-stage">
            {object ? <Suspense fallback={<div className="model-preview-empty">{t("Loading preview…")}</div>}><PreviewCanvas object={object} fit={fit} /></Suspense>
              : <div className="model-preview-empty">{(view === "candidate" && (preview.pending || preview.waiting)) || (view === "current" && baseline.pending) ? <LoaderCircle size={30} className="preview-spinner" /> : <Box size={30} />}
                <span>{view === "current" ? baseline.pending ? t("Loading preview…") : baseline.error ? t("Saved model preview could not be loaded") : t("No saved 3D preview is available for the source revision.")
                  : !preview.available ? t("3D preview is available in the native desktop app.") : finding ? t(finding.message) : t("Building draft preview…")}</span></div>}
          </div>
          <p className="candidate-muted">{t("Rotate with the left mouse button, pan with the right button, and zoom with the wheel.")}</p>
          <div role="status" className="model-preview-status">{ready ? <><Check size={15} />{t("Candidate geometry is ready for review")}</>
            : geometryReady ? <><AlertCircle size={15} />{t("AI candidate does not satisfy the expected checks.")}</>
            : finding ? <><AlertCircle size={15} />{t(finding.message)}</>
            : validBase && preview.available ? <><LoaderCircle size={15} className="preview-spinner" />{t("Building draft preview…")}</> : null}</div>
          {finding && <><Button disabled={preview.pending || !validBase} onClick={() => setRefresh(value => value + 1)}><RotateCw size={15} />{t("Retry candidate preview")}</Button><pre className="candidate-error-detail">{finding.detail}</pre></>}
          {geometryReady && <><h4 className="candidate-properties-title">{t("AI candidate properties")}</h4><dl className="model-preview-metrics">
            <div><dt>{t("Preview volume")}</dt><dd>{number(preview.preview!.metrics.volumeMm3, 3)} {t("mm³")}</dd></div>
            <div><dt>{t("Preview surface area")}</dt><dd>{number(preview.preview!.metrics.areaMm2, 3)} {t("mm²")}</dd></div>
            <div><dt>{t("Preview bounds")}</dt><dd>{preview.preview!.metrics.boundsMm.map(value => number(value, 2)).join(" × ")} {t("mm")}</dd></div>
          </dl></>}
        </section>
      </div>
    </div>
    <div className="model-editor-footer"><small><Check size={13} />{t("Accepting saves one new revision after geometry validation.")}</small><div className="modal-actions">
      <Button onClick={() => onDecision(false)}><X size={15} />{t("Reject candidate")}</Button>
      <Button className="primary" disabled={!ready} onClick={() => { if (ready) onDecision(true); }}><Check size={15} />{t("Accept candidate")}</Button>
    </div></div>
  </>;
}
