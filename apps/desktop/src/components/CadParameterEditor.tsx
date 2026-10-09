import { PersistentDetails, useEditorPreference } from "../features/model-editor/editorPreferences";
import { SlidersHorizontal,ChevronDown,Link2,Trash2,Plus } from "lucide-react";
import { t,number } from "../i18n";
import type { TypedCadDocument } from "../lib/typedCadDocument";
import { addCadParameter,parameterReferences } from "../lib/cadParameters";
import "./CadParameterEditor.css";
export function CadParameterEditor({document,disabled,onChange}:{document:TypedCadDocument;disabled:boolean;onChange:(next:TypedCadDocument)=>void}){
 const [name,setName]=useEditorPreference("parameter.name",""),[value,setValue]=useEditorPreference("parameter.value",10);const next=addCadParameter(document,name,value);
 return <PersistentDetails stateId="parameters" className="cad-parameter-editor" initialOpen={document.parameters.length>0}>
  <summary><SlidersHorizontal size={16}/><span>{t("Named CAD parameters")}</span><span className="cad-section-count">{number(document.parameters.length,0)}</span><ChevronDown size={15} className="cad-section-chevron"/></summary>
  <div className="cad-parameter-content"><p className="field-hint">{t("Bound dimensions rebuild from these parameter values; parameter IDs stay unchanged.")}</p>
   <div className="cad-parameter-list">{document.parameters.map(parameter=>{
    const references=parameterReferences(document,parameter.id);
    return <div key={parameter.id} className="cad-parameter-row">
     <div className="cad-parameter-name"><strong>{parameter.name}</strong><code>{parameter.id}</code></div>
     <label className="cad-parameter-value"><input type="number" step="any" min={-10000} max={10000} value={parameter.valueMm} disabled={disabled} aria-label={t("Parameter {{value0}} in mm",{value0:parameter.id})} onChange={event=>{const mm=event.target.valueAsNumber;if(Number.isFinite(mm)&&Math.abs(mm)<=10000)onChange({...document,parameters:document.parameters.map(item=>item.id===parameter.id?{...item,valueMm:mm}:item)});}}/><span>{t("mm")}</span></label>
     <div className="cad-parameter-references" title={references.join(", ")}><Link2 size={13}/><span>{references.length?references.join(", "):t("No dimension bindings")}</span></div>
     <span title={references.length?t("Unbind this parameter before deleting it."):t("Remove parameter")}><button className="cad-icon-button" type="button" disabled={disabled||references.length>0} aria-label={t("Remove parameter {{value0}}",{value0:parameter.id})} onClick={()=>onChange({...document,parameters:document.parameters.filter(item=>item.id!==parameter.id)})}><Trash2 size={15}/></button></span>
    </div>;
   })}</div>
   <div className="cad-parameter-add"><label>{t("New parameter name")}<input value={name} maxLength={120} disabled={disabled} onChange={event=>setName(event.target.value)}/></label>
    <label>{t("New parameter value in mm")}<input type="number" step="any" min={-10000} max={10000} value={value} disabled={disabled} onChange={event=>{if(Number.isFinite(event.target.valueAsNumber))setValue(event.target.valueAsNumber);}}/></label>
    <button type="button" disabled={disabled||!next} onClick={()=>{if(next){onChange(next);setName("");}}}><Plus size={15}/>{t("Add parameter")}</button>
   </div>
  </div>
 </PersistentDetails>;
}
