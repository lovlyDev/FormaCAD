import { pointIsBound } from "../../lib/sketchBindings";
import { useEffect,useState } from "react";
import { useEditorPreference } from "../../features/model-editor/editorPreferences";
import { Scan } from "lucide-react";
import { t } from "../../i18n";
import type { SketchOperation } from "../../lib/sketchDocument";
import { useSketchDrag } from "./interaction/useSketchDrag";
function framing(points:SketchOperation["points"]){
 const xs=points.map(p=>p.xMm),ys=points.map(p=>p.yMm);
 return {x:Math.min(...xs)-12,y:-Math.max(...ys)-12,width:Math.max(24,Math.max(...xs)-Math.min(...xs)+24),height:Math.max(24,Math.max(...ys)-Math.min(...ys)+24)};
}
export function SketchCanvas({operation,disabled,previewPoints,onChange,featureId="sketch",solvedOutline=null}:{solvedOutline?:SketchOperation["points"]|null;featureId?:string;operation:SketchOperation;disabled:boolean;previewPoints:SketchOperation["points"]|null;onChange:(next:SketchOperation)=>void}){
 const drag=useSketchDrag(operation,disabled||!!previewPoints,onChange);
 const points=(previewPoints??operation.points).map(p=>drag.point?.id===p.id?drag.point:p),byId=new Map(points.map(p=>[p.id,p]));
 const [initialFrame]=useState(()=>framing(points));
 const [frame,setFrame]=useEditorPreference(`sketch.${featureId}.frame`,initialFrame);
 useEffect(()=>{setFrame(current=>current);},[setFrame]);
 return <div className="sketch-canvas-module"><div className="sketch-canvas-toolbar"><button type="button" disabled={drag.active} aria-label={t("Fit sketch to view")} onClick={()=>setFrame(framing(points))}><Scan size={14}/>{t("Fit sketch to view")}</button></div>
 <svg className="sketch-editor-canvas" viewBox={`${frame.x} ${frame.y} ${frame.width} ${frame.height}`} role="img" aria-label={t("Sketch preview")} data-dragging={drag.active}
 onPointerMove={drag.move} onPointerUp={drag.end} onPointerCancel={drag.cancel} onLostPointerCapture={drag.cancel}>
  <line className="sketch-editor-axis" x1={frame.x} y1={0} x2={frame.x+frame.width} y2={0}/><line className="sketch-editor-axis" x1={0} y1={frame.y} x2={0} y2={frame.y+frame.height}/>
  {operation.lines.map(line=>{const a=byId.get(line.startPointId),b=byId.get(line.endPointId);return a&&b?<line key={line.id} className={line.construction?"sketch-editor-construction":undefined} x1={a.xMm} y1={-a.yMm} x2={b.xMm} y2={-b.yMm}/>:null;})}
  {solvedOutline&&operation.lines.filter(line=>!line.construction).map(line=>{const a=solvedOutline.find(point=>point.id===line.startPointId),b=solvedOutline.find(point=>point.id===line.endPointId);return a&&b?<line key={`solved-${line.id}`} className="sketch-solved-outline" x1={a.xMm} y1={-a.yMm} x2={b.xMm} y2={-b.yMm}/>:null;})}
  {points.map(point=><circle key={point.id} data-bound={pointIsBound(operation,point.id)} cx={point.xMm} cy={-point.yMm} r={Math.max(1.2,Math.min(frame.width,frame.height)/45)} tabIndex={disabled||previewPoints||pointIsBound(operation,point.id)?-1:0} aria-label={t("Sketch point {{value0}}",{value0:point.id})} onPointerDown={event=>{if(!pointIsBound(operation,point.id))drag.begin(event,point);}}/>)}
 </svg></div>;
}
