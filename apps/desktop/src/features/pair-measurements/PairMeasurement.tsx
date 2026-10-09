import { useState } from "react";
import { Ruler,MousePointer2,RefreshCw,X,CheckCircle2,LoaderCircle,AlertCircle } from "lucide-react";
import { Button,Select } from "../../components/ui";
import { t,errorText } from "../../i18n";
import { useWorkspace } from "../../stores/workspace";
import { usePersistentState } from "../../lib/persistence";
import { EditorDisclosure } from "../model-editor/EditorDisclosure";
import { pairKinds } from "./pairCapture";
import type { PairKind } from "./pairSchema";
import { usePairSlots } from "./usePairSlots";
import { usePairMeasurement } from "./usePairMeasurement";
import { pairRows } from "./pairRows";
import "./PairMeasurement.css";

/** Uses actual workspace selections; capture buttons never fabricate selection or geometry. */
export function PairMeasurement({bodyId,sceneToken,interactive,hostAvailable}:{bodyId:string|null;sceneToken:string;interactive:boolean;hostAvailable:boolean}){
 const project=useWorkspace(state=>state.project),edge=useWorkspace(state=>state.selectedEdge),face=useWorkspace(state=>state.selectedFace);
 const slots=usePairSlots(project,bodyId,sceneToken,interactive&&hostAvailable,edge,face);
 const [kind,setKind]=useState<PairKind>("minimumDistance");
 const kinds=pairKinds(slots.first,slots.second),activeKind=kinds.includes(kind)?kind:(kinds[0]??"minimumDistance");
 const measurement=usePairMeasurement(slots.scope,slots.first,slots.second,activeKind);
 const [open,setOpen]=usePersistentState(`forma.ui.project.${project?.id??"empty"}.pairMeasurement.open`,false);
 const labels:Record<PairKind,string>={minimumDistance:t("Minimum distance between references"),faceNormalAngle:t("Outward face-normal angle (0–180°)"),edgeAcuteAngle:t("Acute straight-edge angle (0–90°)")};
 const pending=measurement.phase==="pending",previous=!!measurement.report&&(pending||measurement.phase==="failed");
 const rows=measurement.report?pairRows(measurement.report.result,project?.units??"mm"):[];
 if(!hostAvailable)return null;
 return <EditorDisclosure icon={Ruler} title={t("Exact pair measurements")} hint={t("Read only")} open={open} onOpenChange={setOpen} className="model-editor-source pair-measurement">
  <div className="pair-measurement-content">
   <p>{t("Select a CAD edge or face, capture it as the first reference, then select and capture the second on the same saved body.")}</p>
   <div className="pair-measurement-slots">{(["first","second"] as const).map(slot=><div key={slot} className="pair-measurement-slot">
    <strong>{slot==="first"?t("First reference"):t("Second reference")}</strong>
    <span>{slots[slot]?(slots[slot]!.kind==="edge"?t("Captured CAD edge"):t("Captured CAD face")):t("No captured reference")}</span>
    {slots[slot]&&<small><code>{slots[slot]!.ownerFeatureId} · {slots[slot]!.role}</code></small>}
    <Button disabled={!slots.current} onClick={slot==="first"?slots.captureFirst:slots.captureSecond}><MousePointer2 size={14}/>{slot==="first"?t("Capture first selection"):t("Capture second selection")}</Button>
   </div>)}</div>
   <div className="pair-measurement-controls"><label>{t("Measurement kind")}<Select value={activeKind} disabled={!kinds.length} onChange={event=>setKind(event.target.value as PairKind)}>{kinds.map(value=><option key={value} value={value}>{labels[value]}</option>)}</Select></label>
    <Button disabled={!measurement.available||measurement.working} onClick={()=>{void measurement.request();}}><RefreshCw size={14}/>{t("Measure captured pair")}</Button>
    <Button disabled={!slots.first&&!slots.second} onClick={slots.clear}><X size={14}/>{t("Clear captured pair")}</Button>
    {pending&&<Button onClick={measurement.stopWaiting}><X size={14}/>{t("Stop waiting")}</Button>}
   </div>
   {!slots.current&&<p>{t("Select a supported authored edge or face in the current saved body to capture a reference.")}</p>}
   {pending&&<p role="status"><LoaderCircle size={14} className="spin"/>{t("Measuring the saved body with CAD geometry…")}</p>}
   {measurement.working&&!pending&&<p role="status">{t("The prior CAD request is still running. A new measurement can start when it finishes.")}</p>}
   {measurement.error!==undefined&&<div className="pair-measurement-error" role="alert"><AlertCircle size={14}/>{errorText(measurement.error)}</div>}
   {previous&&<p>{t("Previous result for this same captured pair; the new request has not produced a confirmed value.")}</p>}
   {!!rows.length&&<dl>{rows.map(row=><div key={row.label}><dt>{row.label}</dt><dd>{row.value}</dd></div>)}</dl>}
   {measurement.report&&<p className="pair-measurement-provenance"><CheckCircle2 size={14}/>{t("Computed from the selected authored body rebuilt by OpenCascade.")}</p>}
   <p>{t("Face angles use outward normals. Edge angles use unoriented straight lines; circular-edge and curved-face angles are unavailable.")}</p>
   <p>{t("Minimum witnesses are representative points, not unique anchors. Captured references and results are temporary and are not pinned dimensions.")}</p>
  </div>
 </EditorDisclosure>;
}
