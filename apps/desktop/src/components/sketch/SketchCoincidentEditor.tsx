import { Link2,Plus } from "lucide-react";
import "./SketchTools.css";
import { Select } from "../ui";
import { useEditorPreference } from "../../features/model-editor/editorPreferences";
import { t } from "../../i18n";
import type { SketchOperation } from "../../lib/sketchDocument";
import { coincideSketchPoints } from "../../lib/sketchPointConstraints";
export function SketchCoincidentEditor({operation,disabled,onChange,featureId="sketch"}:{featureId?:string;operation:SketchOperation;disabled:boolean;onChange:(next:SketchOperation)=>void}) {
  const [savedFirst,setFirst]=useEditorPreference(`sketch.${featureId}.coincident.first`,operation.points[0]?.id??""),[savedSecond,setSecond]=useEditorPreference(`sketch.${featureId}.coincident.second`,operation.points[1]?.id??"");
  const first=operation.points.some(point=>point.id===savedFirst)?savedFirst:operation.points[0]?.id??"";
  const second=operation.points.some(point=>point.id===savedSecond)?savedSecond:operation.points[1]?.id??"";
  const next=coincideSketchPoints(operation,first,second);
  return <div className="sketch-tool-card"><h4><Link2 size={15}/>{t("Point coincidence")}</h4><div className="sketch-editor-coincident">
    <label>{t("First coincident point")}<Select aria-label={t("First coincident point")} value={first} disabled={disabled} onChange={(e)=>setFirst(e.target.value)}>{operation.points.map((p)=><option key={p.id} value={p.id}>{p.id}</option>)}</Select></label>
    <label>{t("Second coincident point")}<Select aria-label={t("Second coincident point")} value={second} disabled={disabled} onChange={(e)=>setSecond(e.target.value)}>{operation.points.map((p)=><option key={p.id} value={p.id}>{p.id}</option>)}</Select></label>
    <button type="button" disabled={disabled||!next} onClick={()=>{if(next)onChange(next);}}><Plus size={14}/>{t("Add coincidence")}</button>
  </div></div>;
}
