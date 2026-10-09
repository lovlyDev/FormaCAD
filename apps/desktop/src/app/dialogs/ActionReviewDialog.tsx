import { Check, ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { Button, Modal } from "../../components/ui";
import { t, systemText } from "../../i18n";
import { CandidateReview, type CandidateReviewData } from "../../features/agents/review/CandidateReview";

export type PendingAction = {
  title: string; description: string; detail: string;
  run: () => Promise<void>; id?: string; review?: CandidateReviewData; targetProjectId?: string;
};

export function ActionReviewDialog({ pending, onDecision }: { pending: PendingAction | null; onDecision: (allow: boolean) => void }) {
  // Keep the same content through Radix's exit animation instead of flashing a generic dialog.
  const [retained, setRetained] = useState(pending);
  useEffect(() => {
    if (pending) { setRetained(pending); return; }
    const timer = setTimeout(() => setRetained(null), 250);
    return () => clearTimeout(timer);
  }, [pending]);
  const action = pending ?? retained;
  return <Modal open={!!pending} onClose={() => onDecision(false)}
    title={action?.review ? t("Review AI candidate") : systemText(action?.title ?? t("Review action"))}
    description={action?.review ? action.description : systemText(action?.description ?? "")}
    wide={!!action?.review} className={action?.review ? "model-editor-modal candidate-review-modal" : undefined}>
    {action?.review ? <CandidateReview key={`${action.review.base.id}:${action.review.base.currentRevision}:${action.review.source}`} review={action.review} onDecision={allow => { if (pending) onDecision(allow); }} /> : <>
      <div className="permission-detail"><ShieldCheck size={20} /><pre>{action?.detail}</pre></div>
      <div className="modal-actions"><Button onClick={() => onDecision(false)}>{t("Deny")}</Button>
        <Button className="primary" onClick={() => onDecision(true)}><Check size={15} />{t("Allow once")}</Button></div>
    </>}
  </Modal>;
}
