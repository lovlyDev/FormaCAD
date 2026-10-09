import { RefreshCw,LoaderCircle,MousePointer2 } from "lucide-react";
import { Button,Checkbox } from "../../../../components/ui";
import { t,errorText } from "../../../../i18n";
import { formatMeasurement,type DisplayUnit } from "../../../reference-measurements/measurementRows";
import type { useFaceSection } from "./useFaceSection";
const unitFactor=(unit:DisplayUnit)=>unit==="inch"?25.4:unit==="cm"?10:1;
export function FaceSectionControls({section,offsetMm,onOffsetChange,units,automatic,onAutomaticChange}:{section:ReturnType<typeof useFaceSection>;offsetMm:number;onOffsetChange:(value:number)=>void;units:DisplayUnit;automatic:boolean;onAutomaticChange:(value:boolean)=>void}){
 const unitSymbol=units==="inch"?t("in"):units==="cm"?t("cm"):t("mm");
 return <div className="face-section-controls"><p className="field-hint"><MousePointer2 size={14}/>{t("Select an authored planar CAD face. Its outward normal sets the section direction.")}</p>
  <label>{t("Offset from selected face")} ({unitSymbol})<input type="number" step="any" min={-10000/unitFactor(units)} max={10000/unitFactor(units)} value={offsetMm/unitFactor(units)} onChange={event=>{const next=event.target.valueAsNumber*unitFactor(units);if(Number.isFinite(next)&&Math.abs(next)<=10000)onOffsetChange(next);}}/></label>
  <small>{t("Positive offset follows the outward normal; negative offset enters the body.")}</small>
  <Checkbox checked={automatic} onChange={onAutomaticChange}>{t("Update face section automatically")}</Checkbox>
  {!section.available&&<p role="status">{t("Select a supported planar face in the current saved model. Curved sides and unreferenced faces cannot define this section.")}</p>}
  {section.pending&&<p role="status"><LoaderCircle size={14} className="spin"/>{t("Computing exact section…")}</p>}
  <Button disabled={!section.available||section.working} onClick={section.calculate}><RefreshCw size={14}/>{t("Calculate section")}</Button>
  {section.error!==undefined&&<div role="alert">{errorText(section.error)}</div>}
  {section.report&&<p role="status">{t("Section length")}: {formatMeasurement(section.report.geometry.totalLengthMm,1,units)}<small>{t("The plane comes from the selected CAD face; section curves are computed from the whole saved STEP model.")}</small></p>}
 </div>;
}
