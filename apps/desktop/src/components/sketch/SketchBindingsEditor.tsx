import { useState } from "react";
import { Link2,Plus,Trash2 } from "lucide-react";
import { t,number } from "../../i18n";
import { Select } from "../ui";
import type { SketchOperation } from "../../lib/sketchDocument";
import type { CadParameter } from "../../lib/cadParameters";
import { resolveSketchBindings,unlinkSketchBinding,type SketchBinding } from "../../lib/sketchBindings";
import { sketchId } from "../../lib/sketchGeometry";
import "./SketchBindingsEditor.css";
export function SketchBindingsEditor({operation,parameters,disabled,onChange}:{operation:SketchOperation;parameters:CadParameter[];disabled:boolean;onChange:(next:SketchOperation)=>void}){
 const [target,setTarget]=useState("origin"),[axis,setAxis]=useState<"x"|"y"|"z">("x"),[parameterId,setParameterId]=useState("");
 const pointId=operation.points.some(p=>p.id===target)?target:"origin",parameter=parameters.find(p=>p.id===parameterId)??parameters[0];
 const effectiveAxis=pointId!=="origin"&&axis==="z"?"x":axis;
 const duplicate=operation.bindings?.some(b=>b.axis===effectiveAxis&&(pointId==="origin"?b.target==="origin":b.target==="point"&&b.pointId===pointId));
 const update=(id:string,field:"scale"|"offsetMm",value:number)=>{if(!Number.isFinite(value)||Math.abs(value)>10000)return;onChange({...operation,bindings:operation.bindings?.map(b=>b.id===id?{...b,value:{...b.value,[field]:value}}:b)});};
 const add=()=>{if(!parameter||duplicate)return;let current=operation;try{current=resolveSketchBindings(operation,parameters);}catch{return;}
  const coordinate=pointId==="origin"?current.originMm[({x:0,y:1,z:2})[effectiveAxis]]:current.points.find(p=>p.id===pointId)![effectiveAxis==="x"?"xMm":"yMm"];
  const value={kind:"parameter" as const,parameterId:parameter.id,scale:1,offsetMm:coordinate-parameter.valueMm};
  if(Math.abs(value.offsetMm)>10000)return;
  const binding:SketchBinding=pointId==="origin"?{id:sketchId(operation,"binding"),target:"origin",axis:effectiveAxis,value}:{id:sketchId(operation,"binding"),target:"point",pointId,axis:effectiveAxis as "x"|"y",value};
  onChange({...operation,bindings:[...(operation.bindings??[]),binding]});
 };
 return <section className="sketch-links"><h4><Link2 size={15}/>{t("Coordinate parameter links")}</h4><p className="field-hint">{t("Coordinate = parameter × factor + offset. Linked axes follow shared dimensions; unlinking keeps the current shape.")}</p>
 <div className="sketch-link-add"><label>{t("Link target")}<Select value={pointId} disabled={disabled} onChange={e=>setTarget(e.target.value)}><option value="origin">{t("Sketch origin")}</option>{operation.points.map(p=><option key={p.id} value={p.id}>{p.id}</option>)}</Select></label>
 <label>{t("Axis")}<Select value={effectiveAxis} disabled={disabled} onChange={e=>setAxis(e.target.value as typeof axis)}>{(pointId==="origin"?["x","y","z"]:["x","y"]).map(a=><option key={a} value={a}>{a.toUpperCase()}</option>)}</Select></label>
 <label>{t("Parameter")}<Select value={parameter?.id??""} disabled={disabled||!parameters.length} onChange={e=>setParameterId(e.target.value)}>{parameters.map(p=><option key={p.id} value={p.id}>{p.name}</option>)}</Select></label>
 <button type="button" disabled={disabled||!parameter||duplicate||(operation.bindings?.length??0)+operation.constraints.length>=64} onClick={add}><Plus size={14}/>{t("Link coordinate")}</button></div>
 {!parameters.length&&<p className="field-hint">{t("Add a named CAD parameter above to link a coordinate.")}</p>}
 <div className="sketch-link-list">{operation.bindings?.map(b=><div className="sketch-link-row" key={b.id}><div><strong>{b.target==="origin"?t("Sketch origin"):b.pointId} · {b.axis.toUpperCase()}</strong><small>{parameters.find(p=>p.id===b.value.parameterId)?.name??b.value.parameterId}</small></div>
 <label>{t("Factor")}<input type="number" step="any" disabled={disabled} value={b.value.scale} onChange={e=>update(b.id,"scale",e.target.valueAsNumber)}/></label>
 <label>{t("Offset (mm)")}<input type="number" step="any" disabled={disabled} value={b.value.offsetMm} onChange={e=>update(b.id,"offsetMm",e.target.valueAsNumber)}/></label>
 <span>{number((parameters.find(p=>p.id===b.value.parameterId)?.valueMm??0)*b.value.scale+b.value.offsetMm,3)}</span>
 <button type="button" disabled={disabled} aria-label={t("Unlink coordinate {{value0}}",{value0:b.id})} onClick={()=>{try{onChange(unlinkSketchBinding(operation,parameters,b.id));}catch{onChange({...operation,bindings:operation.bindings?.filter(item=>item.id!==b.id)});}}}><Trash2 size={14}/></button></div>)}</div></section>;
}
