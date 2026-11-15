import {useMemo,useState} from "react";
import {Button,Modal,Select,NumberInput} from "../../components/ui";
import type {Project} from "../../types";
import {t} from "../../i18n";
import {ModelPreviewPanel} from "../model-preview/ModelPreviewPanel";
import {useModelPreview} from "../model-preview/useModelPreview";
import {captureOffsetProfile,offsetProfileBodies,type ProfileJoin} from "./offsetProfileCommand";
export function OffsetProfileDialog({project,bodies,initialTarget,disabled,close,apply}:{project:Project;bodies:readonly{id:string;name:string}[];initialTarget:string;disabled:boolean;close:()=>void;apply:(edit:NonNullable<ReturnType<typeof captureOffsetProfile>>)=>void}){
 const [body,setBody]=useState(initialTarget),[offset,setOffset]=useState("2"),[join,setJoin]=useState<ProfileJoin>("intersection"),[auto,setAuto]=useState(true),[refresh,setRefresh]=useState(0);
 const value=offset.trim()===""?NaN:Number(offset),invalid=!Number.isFinite(value)||Math.abs(value)<=1e-7||Math.abs(value)>10000;
 const target=useMemo(()=>offsetProfileBodies(project).find(target=>target.id===body),[project,body]);
 const draft=useMemo(()=>bodies.some(b=>b.id===body)?captureOffsetProfile(project,body,value,join):null,[project,bodies,body,value,join]);
 const preview=useModelPreview(project.id,draft?.previewSource??"",project.currentRevision,!!draft&&!disabled&&(auto||refresh>0),refresh);
 const ready=!!draft&&!!preview.preview&&!preview.pending&&!preview.waiting&&!preview.error&&!disabled;
 return <Modal open wide onClose={close} title={t("Offset profile")} description={t("Offset the extrusion profile. The source profile stays linked and the distance expression is retained.")}><form onSubmit={event=>{event.preventDefault();if(ready&&draft)apply(draft);}}>
  <label className="field">{t("Target body")}<Select value={body} disabled={disabled} onChange={event=>setBody(event.target.value)}><option value="" disabled>{t("Choose a body")}</option>{bodies.map(b=><option key={b.id} value={b.id}>{b.name+" · "+b.id}</option>)}</Select></label>
  {target&&<p>{t("Source profile: {{value0}}",{value0:target.profile.name+" · "+target.profile.id})}</p>}
  <div className="form-row"><label>{t("Profile offset (mm)")}<NumberInput type="number" value={offset} min={-10000} max={10000} step="any" disabled={disabled} aria-label={t("Profile offset (mm)")} aria-invalid={invalid} aria-describedby={invalid?"profile-offset-error":"profile-offset-help"} onChange={event=>setOffset(event.target.value)}/></label><label>{t("Corner join")}<Select value={join} disabled={disabled} onChange={event=>setJoin(event.target.value as ProfileJoin)}><option value="intersection">{t("Intersect extended edges")}</option><option value="rounded">{t("Round outer corners")}</option></Select></label></div>
  <p id="profile-offset-help" className="model-preview-description">{t("Positive offset expands material; negative offset contracts it. Contours cannot disappear or overlap.")}</p>
  {invalid&&<p id="profile-offset-error" role="alert">{t("Enter a finite nonzero offset between −10000 and 10000 mm.")}</p>}
  <ModelPreviewPanel {...preview} enabled={auto} disabled={disabled||!draft} onEnable={setAuto} onRefresh={()=>setRefresh(n=>n+1)}/>
  <div className="modal-actions"><Button type="button" onClick={close}>{t("Cancel")}</Button><Button className="primary" type="submit" disabled={!ready}>{t("Build")}</Button></div>
 </form></Modal>;
}
