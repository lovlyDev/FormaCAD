import { readTypedCadDocument } from "../../lib/typedCadDocument";
import { Radius,MousePointer2,Plus } from "lucide-react";
import { t,number } from "../../i18n";
import { useWorkspace } from "../../stores/workspace";
import { PersistentDetails,useEditorPreference } from "./editorPreferences";
import { edgeFilletDraft } from "./edgeFilletDraft";
import "./EdgeFilletEditor.css";
export function EdgeFilletEditor({source,savedSource,disabled,onChange}:{source:string;savedSource:string;disabled:boolean;onChange:(source:string)=>void}){
 const selection=useWorkspace(state=>state.selectedEdge),revision=useWorkspace(state=>state.project?.currentRevision??null);
 const [radius,setRadius]=useEditorPreference("fillet.radiusMm",1);
 const target=selection&&selection.revisionId===revision?selection:null;
 const next=target&&source===savedSource?edgeFilletDraft(source,target,revision,radius):null;
 return <PersistentDetails className="edge-fillet-editor" stateId="edgeFillet" initialOpen><summary><Radius size={16}/>{t("Edge fillet")}</summary><div>
  <p><MousePointer2 size={14}/>{target?t("Selected edge {{value0}} · {{value1}}",{value0:number(target.edgeOrdinal,0),value1:target.bodyId}):t("Select a CAD edge in the workspace, then open the model editor.")}</p>
  {!readTypedCadDocument(source)&&<p className="field-hint">{t("Edge fillet requires editable CAD IR v2 operation history.")}</p>}
  {target&&!target.semanticKey&&!target.topologyRef&&<p className="field-hint">{t("This edge has no stable fillet reference yet. Fillet currently supports mapped box edges.")}</p>}
  {target?.topologyRef?.role.startsWith("cylinder-edge:")&&<p className="field-hint">{t("Fillet currently supports mapped box edges. Circular-edge measurements remain available in model properties.")}</p>}
  {target?.topologyRef&&<p className="field-hint">{t("The selected edge is linked to its source operation. Supported rigid transforms and size changes preserve this reference.")}</p>}
  {target&&source!==savedSource&&<p className="field-hint">{t("Build the current draft and select the edge again before adding a fillet.")}</p>}
  <div className="edge-fillet-controls"><label>{t("Fillet radius")} ({t("mm")})<input type="number" min={0} max={10000} step="any" value={radius} disabled={disabled} onChange={event=>{if(Number.isFinite(event.target.valueAsNumber))setRadius(event.target.valueAsNumber);}}/></label>
  <button type="button" disabled={disabled||!next} onClick={()=>{if(next)onChange(next);}}><Plus size={14}/>{t("Add fillet to draft")}</button></div>
  <small>{t("Preview the fillet, then build to save a new revision.")}</small>
 </div></PersistentDetails>;
}
