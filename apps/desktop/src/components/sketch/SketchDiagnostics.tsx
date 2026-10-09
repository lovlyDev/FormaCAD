import { Checkbox } from "../ui";
import { t, fixedNumber } from "../../i18n";
import type { SketchAnalysis } from "../../lib/sketchAnalysis";
export function SketchDiagnostics({available,pending,error,analysis,preview,onPreview,onApply,disabled}:{available:boolean;pending:boolean;error:boolean;analysis:SketchAnalysis|null;preview:boolean;onPreview:(value:boolean)=>void;onApply:()=>void;disabled:boolean}) {
  const status=analysis?.status;
  return <div className="sketch-diagnostics" role="status" aria-label={t("Sketch diagnostics")}>
    {!available?<p>{t("Sketch analysis is available in the desktop app.")}</p>:pending?<p>{t("Solving sketch…")}</p>:error?<p>{t("Sketch analysis failed")}</p>:analysis&&<>
      <strong>{status==="solved"?(analysis.degreesOfFreedom===0?t("Fully constrained sketch"):t("Underconstrained sketch")):status==="conflict"?t("Sketch constraints conflict"):status==="invalidProfile"?t("Sketch profile is invalid"):t("Sketch structure is invalid")}</strong>
      {status==="solved"&&<>
        <span>{t("{{value0}} degrees of freedom",{value0:fixedNumber(analysis.degreesOfFreedom??0,0)})}</span>
        <span>{t("{{value0}} redundant equations",{value0:fixedNumber(analysis.redundantEquations??0,0)})}</span>
        <span>{t("{{value0}} holes · area {{value1}} mm²",{value0:fixedNumber(analysis.holeCount,0),value1:fixedNumber(analysis.profileAreaMm2??0,3)})}</span>
        <Checkbox checked={preview} disabled={disabled} onChange={onPreview}>{t("Show solved sketch")}</Checkbox>
        <button type="button" disabled={disabled} onClick={onApply}>{t("Use solved coordinates")}</button>
        <small>{t("The solved preview does not change the draft or the saved 3D model.")}</small>
      </>}
      {analysis.maxResidualMm!==null&&<span>{t("Maximum residual {{value0}} mm",{value0:fixedNumber(analysis.maxResidualMm,6)})}</span>}
      {status==="conflict"&&<small>{t("Highlighted residuals show unsatisfied equations, not a minimal conflict set.")}</small>}
      {analysis.errorCode&&<code>{analysis.errorCode}</code>}
    </>}
  </div>;
}
