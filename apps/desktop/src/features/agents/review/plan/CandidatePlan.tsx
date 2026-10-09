import { useState } from "react";
import { ClipboardList, Check, AlertCircle, CircleDashed } from "lucide-react";
import { EditorDisclosure } from "../../../model-editor/EditorDisclosure";
import { t, number } from "../../../../i18n";
import type { ReviewPlan } from "./reviewPlan";
import "./CandidatePlan.css";

export function CandidatePlan({ plan, results, checked }: { plan?: ReviewPlan | null; results: boolean[]; checked: boolean }) {
  const [open, setOpen] = useState(true);
  if (!plan) return <p className="candidate-plan-note">{t("This provider supplied no structured review plan. Review the CAD changes and exact geometry before accepting.")}</p>;
  function label(check: ReviewPlan["expectedChecks"][number]) {
    switch (check.type) {
      case "validSolid": return t("Valid CAD solid");
      case "bodyCount": return t("Declared model bodies: {{value0}}", { value0: number(check.count, 0) });
      case "bounds": return t("Expected bounds: {{value0}} mm, tolerance {{value1}} mm", { value0: check.sizeMm.map(value => number(value, 2)).join(" × "), value1: number(check.toleranceMm, 3) });
      case "volume": return t("Expected volume: {{value0}} mm³, tolerance {{value1}} mm³", { value0: number(check.valueMm3, 3), value1: number(check.toleranceMm3, 3) });
    }
  }
  return <EditorDisclosure icon={ClipboardList} title={t("AI review plan")} open={open} onOpenChange={setOpen} className="model-editor-source candidate-plan">
    <div className="candidate-plan-content">
      {plan.assumptions.length > 0 && <div><h4>{t("AI assumptions")}</h4><ul>{plan.assumptions.map((assumption, index) => <li key={index}>{assumption}</li>)}</ul></div>}
      {plan.dimensions.length > 0 && <div><h4>{t("Proposed dimensions")}</h4><dl>{plan.dimensions.map((dimension, index) => <div key={index}><dt>{dimension.name}{dimension.parameterId && <code>{dimension.parameterId}</code>}</dt><dd>{number(dimension.valueMm, 3)} {t("mm")}</dd></div>)}</dl><p>{t("Proposed dimensions are AI statements; geometry checks below verify only their listed metrics.")}</p></div>}
      {plan.affectedBodyIds.length > 0 && <div><h4>{t("Affected body IDs")}</h4><div className="candidate-plan-ids">{plan.affectedBodyIds.map(id => <code key={id}>{id}</code>)}</div></div>}
      <div><h4>{t("Expected geometry checks")}</h4>{plan.expectedChecks.length ? <ul className="candidate-plan-checks">{plan.expectedChecks.map((check, index) => <li key={index} data-check-state={!checked ? "pending" : results[index] ? "passed" : "failed"}>
        {!checked ? <CircleDashed size={14} /> : results[index] ? <Check size={14} /> : <AlertCircle size={14} />}{label(check)}<small>{!checked ? t("Pending verification") : results[index] ? t("Check passed") : t("Check failed")}</small>
      </li>)}</ul> : <p>{t("No expected geometry checks were supplied.")}</p>}</div>
    </div>
  </EditorDisclosure>;
}
