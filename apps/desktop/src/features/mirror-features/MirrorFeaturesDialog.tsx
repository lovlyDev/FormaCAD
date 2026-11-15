import {useMemo,useState} from "react";
import {Button,Modal,Select,NumberInput} from "../../components/ui";
import type {Project} from "../../types";
import {t} from "../../i18n";
import {ModelPreviewPanel} from "../model-preview/ModelPreviewPanel";
import {useModelPreview} from "../model-preview/useModelPreview";
import {captureMirrorFeatures,mirrorRange,type MirrorAxis} from "./mirrorFeaturesCommand";
export function MirrorFeaturesDialog({project,bodies,initialTarget,disabled,close,apply}:{project:Project;bodies:readonly{id:string;name:string}[];initialTarget:string;disabled:boolean;close:()=>void;apply:(edit:NonNullable<ReturnType<typeof captureMirrorFeatures>>)=>void}){
 const [body,setBody]=useState(initialTarget),[first,setFirst]=useState(""),[axis,setAxis]=useState<MirrorAxis>("x"),[offset,setOffset]=useState("0"),[auto,setAuto]=useState(true),[refresh,setRefresh]=useState(0);
 const chain=useMemo(()=>mirrorRange(project,body),[project,body]);
 const draft=useMemo(()=>bodies.some(b=>b.id===body)?captureMirrorFeatures(project,body,first,axis,offset.trim()===""?NaN:Number(offset)):null,[project,bodies,body,first,axis,offset]);
 const preview=useModelPreview(project.id,draft?.previewSource??"",project.currentRevision,!!draft&&!disabled&&(auto||refresh>0),refresh);
 const ready=!!draft&&!!preview.preview&&!preview.pending&&!preview.waiting&&!preview.error&&!disabled;
 const selected=chain.slice(chain.findIndex(f=>f.id===first));
 return <Modal open wide onClose={close} title={t("Mirror features")} description={t("Source operations remain linked. Mirrored material changes must leave one connected solid.")}><form onSubmit={event=>{event.preventDefault();if(ready&&draft)apply(draft);}}>
  <label className="field">{t("Target body")}<Select value={body} disabled={disabled} onChange={event=>{setBody(event.target.value);setFirst("");}}><option value="" disabled>{t("Choose a body")}</option>{bodies.map(b=><option key={b.id} value={b.id}>{b.name+" · "+b.id}</option>)}</Select></label>
  <label className="field">{t("First operation to mirror")}<Select value={first} disabled={disabled||!chain.length} onChange={event=>setFirst(event.target.value)}><option value="" disabled>{t("Choose an operation")}</option>{chain.map(f=><option key={f.id} value={f.id}>{f.name+" · "+f.id}</option>)}</Select></label>
  <p className="model-preview-description">{t("Choose the first operation. All following operations through the active body feature are mirrored together.")}</p>
  {draft&&<p>{t("Operations in the mirror range: {{value0}}",{value0:selected.map(f=>f.name+" · "+f.id).join(" → ")})}</p>}
  <div className="form-row"><label>{t("Plane normal axis")}<Select value={axis} disabled={disabled} onChange={event=>setAxis(event.target.value as MirrorAxis)}><option value="x">{t("X")}</option><option value="y">{t("Y")}</option><option value="z">{t("Z")}</option></Select></label><label>{t("Plane offset (mm)")}<NumberInput type="number" value={offset} min={-10000} max={10000} step="any" disabled={disabled} onChange={event=>setOffset(event.target.value)}/></label></div>
  <ModelPreviewPanel {...preview} enabled={auto} disabled={disabled||!draft} onEnable={setAuto} onRefresh={()=>setRefresh(n=>n+1)}/>
  <div className="modal-actions"><Button type="button" onClick={close}>{t("Cancel")}</Button><Button className="primary" type="submit" disabled={!ready}>{t("Build")}</Button></div>
 </form></Modal>;
}
