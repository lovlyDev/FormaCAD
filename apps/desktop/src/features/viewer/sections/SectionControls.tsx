import { Scissors, LoaderCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { Checkbox, Select } from "../../../components/ui";
import { EditorDisclosure } from "../../model-editor/EditorDisclosure";
import { t, number, errorText } from "../../../i18n";
import type { useSectionView } from "./useSectionView";
import type { VectorMm } from "./sectionPlane";
import "./SectionControls.css";
import { FaceSectionControls } from "./face-section/FaceSectionControls";
export function SectionControls({ section }: { section: ReturnType<typeof useSectionView> }) {
  const [present, setPresent] = useState(section.open);
  useEffect(() => {
    if (section.open) { setPresent(true); return; }
    const timer = setTimeout(() => setPresent(false), 260); return () => clearTimeout(timer);
  }, [section.open]);
  if (!section.open && !present) return null;
  return <div className="section-controls" data-open={section.open}><EditorDisclosure icon={Scissors} title={t("Section view")} open={section.open} onOpenChange={section.setOpen}>
    <div className="section-controls-content" inert={section.suspended}><Checkbox checked={section.enabled} disabled={!section.available} onChange={section.setEnabled}>{t("Clip model with section plane")}</Checkbox>
      <label>{t("Section source")}<Select value={section.mode} onChange={event=>section.setMode(event.target.value as "manual"|"selectedFace")}><option value="manual">{t("Numeric or free plane")}</option><option value="selectedFace">{t("Selected CAD face")}</option></Select></label>
      {section.mode==="selectedFace"?<FaceSectionControls section={section.faceSection} offsetMm={section.faceOffset} onOffsetChange={section.setFaceOffset} automatic={section.faceAutomatic} onAutomaticChange={section.setFaceAutomatic} units={section.units}/>:<>
      <label>{t("Section plane")}<Select value={section.axis} onChange={event => section.setAxis(event.target.value as typeof section.axis)}><option value="xy">{t("XY")}</option><option value="xz">{t("XZ")}</option><option value="yz">{t("YZ")}</option><option value="free">{t("Free plane")}</option></Select></label>
      <label>{t("Plane offset in mm")}<input type="number" step="any" min={-10000} max={10000} value={section.offset} onChange={event => { if (Number.isFinite(event.target.valueAsNumber) && Math.abs(event.target.valueAsNumber) <= 10000) section.setOffset(event.target.valueAsNumber); }} /></label>
      {section.axis === "free" && <fieldset><legend>{t("Plane normal (unitless)")}</legend><div className="section-normal">{section.normal.map((value, index) => <label key={index}>{["X", "Y", "Z"][index]}<input type="number" step="any" min={-1e6} max={1e6} value={value} onChange={event => { const n = event.target.valueAsNumber; if (!Number.isFinite(n) || Math.abs(n) > 1e6) return; const next: VectorMm = [...section.normal]; next[index] = n; section.setNormal(next); }} /></label>)}</div></fieldset>}
      {!section.valid && <p role="alert">{t("Section plane normal must be nonzero.")}</p>}
      <div role="status" className="section-result">{section.pending ? <><LoaderCircle size={14} className="spin" />{t("Computing exact section…")}</> : section.error ? errorText(section.error) : section.report ? <>{t("Exact section length: {{value0}} mm", { value0: number(section.report.geometry.totalLengthMm, 3) })}<small>{section.report.geometry.curves.length ? t("Display curves are sampled; their lengths come from CAD geometry.") : t("The plane does not intersect this model.")}</small></> : null}</div>
      </>}
      <p className="field-hint">{t("Section view does not change the saved model or exported geometry.")}</p>
      <p className="field-hint">{t("The section uses the whole saved STEP model, including hidden bodies.")}</p>
    </div>
  </EditorDisclosure></div>;
}
