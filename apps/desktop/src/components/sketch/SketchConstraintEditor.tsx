import { Trash2 } from "lucide-react";
import { Select } from "../ui";
import { t, fixedNumber } from "../../i18n";
import type { SketchOperation } from "../../lib/sketchDocument";
import type { CadParameter } from "../../lib/cadParameters";
import type { SketchAnalysis } from "../../lib/sketchAnalysis";
import { sketchId } from "../../lib/sketchGeometry";
import { splitProfileLine } from "../../lib/sketchProfileEditing";
export function SketchConstraintEditor({operation,parameters,analysis,disabled,onChange}:{operation:SketchOperation;parameters:CadParameter[];analysis:SketchAnalysis|null;disabled:boolean;onChange:(next:SketchOperation)=>void}) {
  const points=new Map(operation.points.map((point)=>[point.id,point]));
  const add=(kind:"horizontal"|"vertical"|"length",lineId:string)=>{
    if(operation.constraints.length+(operation.bindings?.length??0)>=64)return;
    const line=operation.lines.find((line)=>line.id===lineId);if(!line)return;
    const a=points.get(line.startPointId),b=points.get(line.endPointId);if(!a||!b)return;
    const constraint:SketchOperation["constraints"][number]=kind==="length"?
      {id:sketchId(operation,"constraint"),kind,lineId,distance:{kind:"literal",mm:Math.max(.01,Math.round(Math.hypot(b.xMm-a.xMm,b.yMm-a.yMm)*100)/100)}}:
      {id:sketchId(operation,"constraint"),kind,lineId};
    onChange({...operation,constraints:[...operation.constraints,constraint]});
  };
  const update=(id:string,changes:Partial<SketchOperation["constraints"][number]>)=>onChange({...operation,constraints:operation.constraints.map((constraint)=>constraint.id===id?{...constraint,...changes}:constraint)});
  const residuals=new Map(analysis?.constraints.map((item)=>[item.id,item]));
  return <>
    <div className="sketch-editor-lines">{operation.lines.map((line)=>{
      const existing=operation.constraints.filter((item)=>item.lineId===line.id);
      return <div key={line.id}>
        <strong>{line.id}</strong>
        {(["horizontal","vertical","length"] as const).map((kind)=>!existing.some((item)=>item.kind===kind)&&
          <button key={kind} type="button" disabled={disabled||operation.constraints.length+(operation.bindings?.length??0)>=64} onClick={()=>add(kind,line.id)}>{t(kind)}</button>)}
        {!line.construction&&<button type="button" disabled={disabled||operation.points.length>=32||operation.lines.length>=64}
          aria-label={t("Split edge {{value0}}",{value0:line.id})} title={t("Splitting an edge removes its line constraints.")}
          onClick={()=>{const next=splitProfileLine(operation,line.id);if(next)onChange(next);}}>{t("Split edge")}</button>}
      </div>;
    })}</div>
    <div className="sketch-editor-constraints">{operation.constraints.map((constraint)=>{
      const distance=constraint.kind==="length"?constraint.distance:undefined;
      const residual=residuals.get(constraint.id);
      return <div key={constraint.id} className={residual&&!residual.satisfied?"sketch-constraint-unsatisfied":undefined}>
        <span>{t(constraint.kind)} · {constraint.lineId??constraint.pointId??[constraint.firstPointId,constraint.secondPointId].filter(Boolean).join(" → ")}</span>
        {distance&&<label>{t("Dimension source")}
          <Select aria-label={t("Dimension source {{value0}}",{value0:constraint.id})} disabled={disabled} value={distance.kind==="parameter"?distance.parameterId:""}
            onChange={(event)=>{
              if(event.target.value)update(constraint.id,{distance:{kind:"parameter",parameterId:event.target.value}});
              else update(constraint.id,{distance:{kind:"literal",mm:distance.kind==="literal"?distance.mm:Math.max(.01,parameters.find((p)=>p.id===distance.parameterId)?.valueMm??1)}});
            }}>
            <option value="">{t("Numeric dimension")}</option>
            {parameters.map((parameter)=><option key={parameter.id} value={parameter.id}>{parameter.name} · {parameter.id}</option>)}
            {distance.kind==="parameter"&&!parameters.some((p)=>p.id===distance.parameterId)&&<option value={distance.parameterId}>{distance.parameterId}</option>}
          </Select>
        </label>}
        {distance?.kind==="literal"&&<label>{t("Length in mm")}<input type="number" min={.01} max={10000} step="any" value={distance.mm} disabled={disabled}
          onChange={(e)=>{const mm=e.target.valueAsNumber;if(Number.isFinite(mm)&&mm>0&&mm<=10000)update(constraint.id,{distance:{kind:"literal",mm}});}}/></label>}
        {distance?.kind==="parameter"&&<small>{distance.parameterId} · {fixedNumber(parameters.find((p)=>p.id===distance.parameterId)?.valueMm??0,3)} {t("mm")}</small>}
        {constraint.kind==="fixed"&&(["xMm","yMm"] as const).map((axis)=><label key={axis}>{axis==="xMm"?t("Fixed X"):t("Fixed Y")}
          <input type="number" step="any" value={constraint[axis]??0} disabled={disabled} aria-label={axis==="xMm"?t("Fixed X {{value0}}",{value0:constraint.id}):t("Fixed Y {{value0}}",{value0:constraint.id})}
            onChange={(e)=>{const value=e.target.valueAsNumber;if(Number.isFinite(value)&&Math.abs(value)<=10000)update(constraint.id,{[axis]:value});}}/>
        </label>)}
        {residual&&<small>{t("Residual {{value0}} mm",{value0:fixedNumber(residual.residualMm,6)})}</small>}
        <button type="button" disabled={disabled} aria-label={t("Remove constraint {{value0}}",{value0:constraint.id})}
          onClick={()=>onChange({...operation,constraints:operation.constraints.filter((item)=>item.id!==constraint.id)})}><Trash2 size={14}/></button>
      </div>;
    })}
      <small>{t("{{value0}} points · {{value1}} constraints",{value0:fixedNumber(operation.points.length,0),value1:fixedNumber(operation.constraints.length,0)})}</small>
    </div>
  </>;
}
