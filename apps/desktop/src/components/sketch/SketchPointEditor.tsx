import { pointIsBound } from "../../lib/sketchBindings";
import { t } from "../../i18n";
import type { SketchOperation } from "../../lib/sketchDocument";
import { fixSketchPoint } from "../../lib/sketchPointConstraints";
import { removeProfilePoint } from "../../lib/sketchProfileEditing";
import { sketchLoops } from "../../lib/sketchTopology";
export function SketchPointEditor({operation,disabled,onChange}:{operation:SketchOperation;disabled:boolean;onChange:(next:SketchOperation)=>void}) {
  const loops=sketchLoops(operation);
  const change=(id:string,axis:"xMm"|"yMm",value:number)=>{
    if(!Number.isFinite(value)||Math.abs(value)>10000)return;
    onChange({...operation,points:operation.points.map((point)=>point.id===id?{...point,[axis]:value}:point)});
  };
  return <div className="sketch-editor-points">{operation.points.map((point)=>{
    const fixed=operation.constraints.some((c)=>c.kind==="fixed"&&c.pointId===point.id);
    const removable=loops?.some((loop)=>loop.pointIds.includes(point.id)&&loop.pointIds.length>3);
    return <div key={point.id}>
      <strong>{point.id}</strong>
      <label>{t("X")}<input type="number" step="any" value={point.xMm} disabled={disabled||pointIsBound(operation,point.id,"x")} aria-label={t("Point {{value0}} X",{value0:point.id})} onChange={(e)=>change(point.id,"xMm",e.target.valueAsNumber)}/></label>
      <label>{t("Y")}<input type="number" step="any" value={point.yMm} disabled={disabled||pointIsBound(operation,point.id,"y")} aria-label={t("Point {{value0}} Y",{value0:point.id})} onChange={(e)=>change(point.id,"yMm",e.target.valueAsNumber)}/></label>
      <button type="button" disabled={disabled||fixed||operation.constraints.length+(operation.bindings?.length??0)>=64} aria-label={t("Fix point {{value0}}",{value0:point.id})}
        onClick={()=>{const next=fixSketchPoint(operation,point.id);if(next)onChange(next);}}>{fixed?t("Point is fixed"):t("Fix point")}</button>
      {removable&&<button type="button" disabled={disabled} aria-label={t("Remove point {{value0}}",{value0:point.id})}
        title={t("Removing a vertex clears constraints on its adjacent edges and attached guides.")}
        onClick={()=>{const next=removeProfilePoint(operation,point.id);if(next)onChange(next);}}>{t("Remove point")}</button>}
    </div>;
  })}</div>;
}
