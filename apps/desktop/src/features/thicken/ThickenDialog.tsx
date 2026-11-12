import { useMemo,useState } from "react";
import { Button,Modal,NumberInput } from "../../components/ui";
import type { Project } from "../../types";
import { t } from "../../i18n";
import { ModelPreviewPanel } from "../model-preview/ModelPreviewPanel";
import { useModelPreview } from "../model-preview/useModelPreview";
import { captureThicken,type ThickenIntent } from "./thickenCommand";
export function ThickenDialog({project,intent,disabled,close,apply}:{project:Project;intent:ThickenIntent;disabled:boolean;close:()=>void;apply:(edit:NonNullable<ReturnType<typeof captureThicken>>)=>void}){
 const [offset,setOffset]=useState("2"),[auto,setAuto]=useState(true),[refresh,setRefresh]=useState(0);
 const draft=useMemo(()=>captureThicken(project,intent,offset.trim()===""?NaN:Number(offset)),[project,intent,offset]);
 const preview=useModelPreview(project.id,draft?.previewSource??"",project.currentRevision,!!draft&&!disabled&&(auto||refresh>0),refresh);
 const ready=!!draft&&!!preview.preview&&!preview.pending&&!preview.waiting&&!preview.error&&!disabled;
 return <Modal open wide onClose={close} title={t("Thicken face")} description={t("The original body remains. A new body is created from the selected face.")}><form onSubmit={event=>{event.preventDefault();if(ready&&draft)apply(draft);}}>
  <p>{t("Selected face: {{value0}}",{value0:intent.bodyId+" · "+intent.reference.role})}</p>
  <label className="field">{t("Signed thickness (mm)")}<NumberInput type="number" value={offset} min={-10000} max={10000} step={.1} disabled={disabled} onChange={event=>setOffset(event.target.value)}/></label>
  <p className="model-preview-description">{t("Positive thickness follows the selected face normal. Negative thickness reverses it.")}</p>
  <ModelPreviewPanel {...preview} enabled={auto} disabled={disabled||!draft} onEnable={setAuto} onRefresh={()=>setRefresh(value=>value+1)}/>
  <div className="modal-actions"><Button type="button" onClick={close}>{t("Cancel")}</Button><Button className="primary" type="submit" disabled={!ready}>{t("Build")}</Button></div>
 </form></Modal>;
}
