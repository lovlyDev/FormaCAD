import { resolveSketchBindings } from "../lib/sketchBindings";
import { SketchBindingsEditor } from "./sketch/SketchBindingsEditor";
import { shapeDifference } from "../features/model-editor/sketch-review/shapeDifference";
import { AlertTriangle,ChevronRight,PenLine } from "lucide-react";
import { Select } from "./ui";
import { PersistentDetails,useEditorPreference } from "../features/model-editor/editorPreferences";
import { t } from "../i18n";
import type { SketchOperation } from "../lib/sketchDocument";
import type { CadParameter } from "../lib/cadParameters";
import { SketchConstructionEditor } from "./sketch/SketchConstructionEditor";
import { SketchCanvas } from "./sketch/SketchCanvas";
import { SketchProfileEditor } from "./sketch/SketchProfileEditor";
import { SketchPointEditor } from "./sketch/SketchPointEditor";
import { SketchCoincidentEditor } from "./sketch/SketchCoincidentEditor";
import { SketchConstraintEditor } from "./sketch/SketchConstraintEditor";
import { SketchDiagnostics } from "./sketch/SketchDiagnostics";
import { useSketchAnalysis } from "./sketch/useSketchAnalysis";
import "./Sketch2dEditor.css";
const NO_PARAMETERS:CadParameter[]=[];
export function Sketch2dEditor({operation,disabled,onChange,parameters=NO_PARAMETERS,featureId="sketch",usage=[]}:{usage?:string[];operation:SketchOperation;disabled:boolean;onChange:(next:SketchOperation)=>void;parameters?:CadParameter[];featureId?:string}) {
  let resolved=operation;let bindingError=false;try{resolved=resolveSketchBindings(operation,parameters);}catch{bindingError=true;}
  const [preview,setPreview]=useEditorPreference(`sketch.${featureId}.solvedPreview`,false);
  const diagnostics=useSketchAnalysis(operation,parameters,featureId);
  const solved=diagnostics.analysis?.status==="solved"?diagnostics.analysis.solvedPoints:null;
  return <PersistentDetails stateId={`sketch.${featureId}`} className="sketch-editor">
    <summary><span className="sketch-disclosure-icon"><ChevronRight className="sketch-disclosure-chevron" size={14}/><PenLine size={14}/></span><span>{t("Edit 2D sketch")}</span></summary>
    <div className="sketch-editor-plane">
      <label>{t("Sketch plane")}<Select value={operation.plane} disabled={disabled} onChange={(e)=>onChange({...operation,plane:e.target.value as SketchOperation["plane"]})}>
        <option value="xy">{t("XY")}</option><option value="xz">{t("XZ")}</option><option value="yz">{t("YZ")}</option>
      </Select></label>
      {resolved.originMm.map((value,axis)=><label key={axis}>{t("Plane origin {{value0}}",{value0:["X","Y","Z"][axis]})}
        <input type="number" step="any" value={value} disabled={disabled||operation.bindings?.some(b=>b.target==="origin"&&b.axis===["x","y","z"][axis])} onChange={(e)=>{
          const value=e.target.valueAsNumber;if(!Number.isFinite(value)||Math.abs(value)>10000)return;
          const originMm=[...operation.originMm] as SketchOperation["originMm"];originMm[axis]=value;onChange({...operation,originMm});
        }}/>
      </label>)}
    </div>
    {usage.length>0&&<p className="sketch-editor-hint">{t("This profile is used by: {{value0}}",{value0:usage.join(", ")})}</p>}
    <p className="sketch-editor-hint">{t("The 3D preview shows the complete model, including other profiles and operations.")}</p>
    <SketchDiagnostics {...diagnostics} preview={preview} onPreview={setPreview} disabled={disabled}
      onApply={()=>{if(solved){onChange({...operation,points:solved});setPreview(false);}}}/>
    {solved&&shapeDifference(resolved.points,solved)&&<p className="sketch-shape-warning"><AlertTriangle size={15}/>{t("Dashed lines show the profile that constraints will build.")}</p>}
    <SketchCanvas featureId={featureId} operation={resolved} disabled={disabled||bindingError} solvedOutline={!preview&&solved&&shapeDifference(resolved.points,solved)?solved:null} previewPoints={preview?solved:null} onChange={onChange}/>
    <p className="sketch-editor-hint">{preview&&solved?t("Solved sketch preview; use solved coordinates before dragging."):t("Drag points to change the profile; constraints are solved when the model builds.")}</p>
    {bindingError&&<p role="alert">{t("Coordinate links are invalid; check parameters and targets.")}</p>}
    <SketchBindingsEditor operation={operation} parameters={parameters} disabled={disabled} onChange={onChange}/>
    <SketchProfileEditor featureId={featureId} operation={resolved} disabled={disabled||bindingError} onChange={onChange}/>
    <SketchPointEditor operation={resolved} disabled={disabled||bindingError} onChange={onChange}/>
    <SketchConstructionEditor featureId={featureId} operation={resolved} disabled={disabled||bindingError} onChange={onChange}/>
    <SketchCoincidentEditor featureId={featureId} operation={resolved} disabled={disabled||bindingError} onChange={onChange}/>
    <SketchConstraintEditor operation={operation} parameters={parameters} analysis={diagnostics.analysis} disabled={disabled} onChange={onChange}/>
  </PersistentDetails>;
}
