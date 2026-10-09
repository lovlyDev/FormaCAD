import { useMemo,useState } from "react";
import { AlertCircle,CheckCircle2,Code2,LoaderCircle,Ruler,RefreshCw,X } from "lucide-react";
import { Button,Select } from "../../components/ui";
import { t,errorText,rawError } from "../../i18n";
import type { Project } from "../../types";
import type { EdgeSelection } from "../viewer/edgeSelection";
import type { FaceSelection } from "../viewer/faceSelection";
import { EditorDisclosure } from "../model-editor/EditorDisclosure";
import { usePersistentState } from "../../lib/persistence";
import { supportedMeasurementQueries } from "./measurementCapabilities";
import { useExactReferenceMeasurement } from "./useExactReferenceMeasurement";
import { measurementRows } from "./measurementRows";
import type { MeasurementQuery } from "./measurementSchema";
import "./ExactReferenceMeasurement.css";

export function ExactReferenceMeasurement({project,bodyId,edge,face,interactive,hostAvailable}:{project:Project|null;bodyId:string|null;edge:EdgeSelection|null;face:FaceSelection|null;interactive:boolean;hostAvailable:boolean}){
  const [open,setOpen]=usePersistentState(`forma.ui.project.${project?.id??"empty"}.referenceMeasurement.open`,false);
  const [kind,setKind]=useState<MeasurementQuery["kind"]>("bodyMetrics");
  const [diagnosticsOpen,setDiagnosticsOpen]=useState(false);
  const queries=useMemo(()=>supportedMeasurementQueries(project,bodyId,edge,face),[project,bodyId,edge,face]);
  const query=queries.find(query=>query.kind===kind)??queries[0]??{kind:"bodyMetrics" as const};
  const selectionToken=JSON.stringify({edge,face,bodyId});
  const measurement=useExactReferenceMeasurement({project,bodyId,query,selectionToken,interactive:interactive&&hostAvailable&&queries.length>0});
  const labels:Record<MeasurementQuery["kind"],string>={bodyMetrics:t("Selected body metrics"),edgeLength:t("Selected edge length"),edgeRadius:t("Selected edge radius"),edgeDiameter:t("Selected edge diameter"),faceArea:t("Selected face area"),planarFace:t("Selected planar face")};
  const pending=measurement.phase==="pending",previous=!!measurement.report&&(pending||measurement.phase==="failed");
  const rows=measurement.report?measurementRows(measurement.report.result,project?.units??"mm"):[];
  if(!hostAvailable||!queries.length)return null;
  return <EditorDisclosure icon={Ruler} title={t("Exact selection measurements")} hint={t("Read only")} open={open} onOpenChange={setOpen} className="model-editor-source reference-measurement">
    <div className="reference-measurement-content">
      <div className="reference-measurement-controls"><label>{t("Measurement kind")}<Select value={query.kind} disabled={!measurement.available||pending} onChange={event=>setKind(event.target.value as MeasurementQuery["kind"])}>{queries.map(query=><option value={query.kind} key={query.kind}>{labels[query.kind]}</option>)}</Select></label>
        <Button disabled={!measurement.available||measurement.working} onClick={()=>{void measurement.request();}}><RefreshCw size={14}/>{t("Measure selection")}</Button>
        {pending&&<Button onClick={measurement.stopWaiting}><X size={14}/>{t("Stop waiting")}</Button>}
      </div>
      {pending&&<p role="status"><LoaderCircle size={14} className="spin"/>{t("Measuring the saved body with CAD geometry…")}</p>}
      {measurement.working&&!pending&&<p role="status"><LoaderCircle size={14} className="spin"/>{t("The prior CAD request is still running. A new measurement can start when it finishes.")}</p>}
      {measurement.error!==undefined&&<div className="reference-measurement-error" role="alert"><AlertCircle size={14}/><span>{errorText(measurement.error)}</span></div>}
      {previous&&<p className="reference-measurement-previous">{t("Previous result for this same saved selection; the new request has not produced a confirmed value.")}</p>}
      {!!rows.length&&<dl>{rows.map(row=><div key={row.label}><dt>{row.label}</dt><dd>{row.value}</dd></div>)}</dl>}
      {measurement.report&&<p className="reference-measurement-provenance"><CheckCircle2 size={14}/>{t("Computed from the selected authored body rebuilt by OpenCascade.")}<small>{t("Body: {{value0}} · revision: {{value1}}",{value0:measurement.report.bodyId,value1:measurement.report.revisionId})}</small></p>}
      <p className="field-hint">{t("These measurements do not change the saved model and are not pinned dimensions.")}</p>
      {edge&&queries.some(query=>query.kind==="edgeLength")&&!queries.some(query=>query.kind==="edgeRadius")&&<p className="field-hint">{t("This authored edge is straight; circular radius measurement is unavailable.")}</p>}
      {measurement.error!==undefined&&<EditorDisclosure icon={Code2} title={t("Technical details")} open={diagnosticsOpen} onOpenChange={setDiagnosticsOpen} className="model-editor-source reference-measurement-diagnostic"><pre>{rawError(measurement.error)}</pre></EditorDisclosure>}
    </div>
  </EditorDisclosure>;
}
