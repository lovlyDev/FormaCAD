import { useMemo,useState } from "react";
import { Button,Modal,Select,NumberInput } from "../../components/ui";
import type { Project } from "../../types";
import { t } from "../../i18n";
import { ModelPreviewPanel } from "../model-preview/ModelPreviewPanel";
import { useModelPreview } from "../model-preview/useModelPreview";
import { captureSplit,type SplitAxis } from "./splitCommand";
export function SplitDialog({project,bodies,initialTarget,disabled,close,apply}:{project:Project;bodies:readonly{id:string;name:string}[];initialTarget:string;disabled:boolean;close:()=>void;apply:(edit:NonNullable<ReturnType<typeof captureSplit>>)=>void}){
 const [body,setBody]=useState(initialTarget),[axis,setAxis]=useState<SplitAxis>("x"),[offset,setOffset]=useState("0"),[auto,setAuto]=useState(true),[refresh,setRefresh]=useState(0);
 const draft=useMemo(()=>captureSplit(project,body,axis,offset.trim()===""?NaN:Number(offset)),[project,body,axis,offset]);
 const preview=useModelPreview(project.id,draft?.previewSource??"",project.currentRevision,!!draft&&!disabled&&(auto||refresh>0),refresh);
 const ready=!!draft&&!!preview.preview&&!preview.pending&&!preview.waiting&&!preview.error&&!disabled;
 return <Modal open wide onClose={close} title={t("Split body by plane")} description={t("Split creates two named bodies in one revision.")}><form onSubmit={event=>{event.preventDefault();if(ready&&draft)apply(draft);}}>
 <label className="field">{t("Target body")}<Select value={body} onChange={e=>setBody(e.target.value)} disabled={disabled}><option value="" disabled>{t("Choose a body")}</option>{bodies.map(item=><option key={item.id} value={item.id}>{item.name+" · "+item.id}</option>)}</Select></label>
 <div className="form-row"><label>{t("Plane normal axis")}<Select value={axis} disabled={disabled} onChange={e=>setAxis(e.target.value as SplitAxis)}><option value="x">{t("X")}</option><option value="y">{t("Y")}</option><option value="z">{t("Z")}</option></Select></label><label>{t("Plane offset (mm)")}<NumberInput type="number" value={offset} min={-10000} max={10000} step={1} disabled={disabled} onChange={e=>setOffset(e.target.value)}/></label></div>
 <p className="model-preview-description">{t("The original body keeps the positive half. A new body contains the negative half. Tangent and missed planes are rejected.")}</p>
 <ModelPreviewPanel {...preview} enabled={auto} disabled={disabled||!draft} onEnable={setAuto} onRefresh={()=>setRefresh(value=>value+1)}/>
 <div className="modal-actions"><Button type="button" onClick={close}>{t("Cancel")}</Button><Button className="primary" type="submit" disabled={!ready}>{t("Build")}</Button></div>
 </form></Modal>;
}
