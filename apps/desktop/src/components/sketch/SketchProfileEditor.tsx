import { RectangleHorizontal,Plus } from "lucide-react";
import "./SketchTools.css";
import { useEditorPreference } from "../../features/model-editor/editorPreferences";
import { t, fixedNumber } from "../../i18n";
import type { SketchOperation } from "../../lib/sketchDocument";
import { addRectangleContour, removeProfileContour } from "../../lib/sketchProfileEditing";
import { sketchLoops } from "../../lib/sketchTopology";
export function SketchProfileEditor({operation,disabled,onChange,featureId="sketch"}:{featureId?:string;operation:SketchOperation;disabled:boolean;onChange:(next:SketchOperation)=>void}) {
  const [rectangle,setRectangle]=useEditorPreference(`sketch.${featureId}.rectangle`,{x:-4,y:-3,width:8,height:6});
  const loops=sketchLoops(operation);
  const next=addRectangleContour(operation,rectangle.x,rectangle.y,rectangle.width,rectangle.height);
  return <div className="sketch-editor-profile-tools sketch-tool-card">
    <h4><RectangleHorizontal size={15}/>{t("Profile contours")}</h4>
    <p className="sketch-editor-hint">{t("Inner closed contours become holes. Contours must not touch or intersect.")}</p>
    <div className="sketch-editor-profile-add">
      {(["x","y","width","height"] as const).map((field)=><label key={field}>{t({x:"Contour X",y:"Contour Y",width:"Contour width",height:"Contour height"}[field])}
        <input type="number" step="any" min={field==="width"||field==="height"?0.01:-10000} max={10000} value={rectangle[field]} disabled={disabled}
          onChange={(event)=>{if(Number.isFinite(event.target.valueAsNumber))setRectangle({...rectangle,[field]:event.target.valueAsNumber});}} />
      </label>)}
      <button type="button" disabled={disabled||!next} onClick={()=>{if(next)onChange(next);}}><Plus size={14}/>{t("Add rectangle contour")}</button>
    </div>
    {loops?.map((loop)=><div className="sketch-editor-profile-row" key={loop.id}>
      <span>{loop.id} · {t("{{value0}} contour points",{value0:fixedNumber(loop.pointIds.length,0)})}</span>
      {loops.length>1&&<button type="button" disabled={disabled} aria-label={t("Remove contour {{value0}}",{value0:loop.id})}
        onClick={()=>{const result=removeProfileContour(operation,loop.id);if(result)onChange(result);}}>{t("Remove contour")}</button>}
    </div>)}
    {!loops&&<p className="sketch-editor-hint">{t("Sketch contour topology is invalid")}</p>}
    <small>{t("{{value0}} contours",{value0:fixedNumber(loops?.length??0,0)})}</small>
  </div>;
}
