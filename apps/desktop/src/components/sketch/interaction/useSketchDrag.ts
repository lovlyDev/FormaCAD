import { useEffect,useRef,useState,type PointerEvent } from "react";
import type { SketchOperation } from "../../../lib/sketchDocument";
import { SketchDragSession,type SketchPoint } from "./dragSession";
export function useSketchDrag(operation:SketchOperation,disabled:boolean,onChange:(next:SketchOperation)=>void){
 const latest=useRef({operation,onChange});latest.current={operation,onChange};
 const session=useRef<SketchDragSession|null>(null),canvas=useRef<SVGSVGElement|null>(null),frame=useRef<number|null>(null);
 const [point,setPoint]=useState<SketchPoint|null>(null);
 function finish(commit:boolean){
  const active=session.current;session.current=null;if(frame.current!==null){cancelAnimationFrame(frame.current);frame.current=null;}
  if(active&&canvas.current?.hasPointerCapture(active.pointerId))canvas.current.releasePointerCapture(active.pointerId);
  if(active&&commit&&active.changed)latest.current.onChange(active.apply(latest.current.operation));
  setPoint(null);
 }
 useEffect(()=>{
  const blur=()=>finish(true);const key=(event:KeyboardEvent)=>{if(event.key==="Escape"&&session.current){event.preventDefault();event.stopPropagation();finish(false);}};
  window.addEventListener("blur",blur);document.addEventListener("keydown",key,true);
  return()=>{window.removeEventListener("blur",blur);document.removeEventListener("keydown",key,true);if(frame.current!==null)cancelAnimationFrame(frame.current);session.current=null;};
 },[]);
 useEffect(()=>{if(disabled&&session.current)finish(false);},[disabled]);
 const begin=(event:PointerEvent<SVGCircleElement>,point:SketchPoint)=>{
  if(disabled||event.button!==0||session.current)return;
  const svg=event.currentTarget.ownerSVGElement,matrix=svg?.getScreenCTM();if(!svg||!matrix)return;
  event.preventDefault();event.stopPropagation();canvas.current=svg;
  session.current=new SketchDragSession(event.pointerId,point,event.clientX,event.clientY,matrix.inverse());
  svg.setPointerCapture(event.pointerId);setPoint(point);
 };
 const move=(event:PointerEvent<SVGSVGElement>)=>{
  const active=session.current;if(!active||event.pointerId!==active.pointerId)return;
  active.move(event.pointerId,event.clientX,event.clientY);
  if(frame.current===null)frame.current=requestAnimationFrame(()=>{frame.current=null;if(session.current)setPoint({...session.current.current});});
 };
 const end=(event:PointerEvent<SVGSVGElement>)=>{if(event.pointerId===session.current?.pointerId){session.current.move(event.pointerId,event.clientX,event.clientY);finish(true);}};
 const cancel=(event:PointerEvent<SVGSVGElement>)=>{if(event.pointerId===session.current?.pointerId)finish(false);};
 return {point,begin,move,end,cancel,active:!!point};
}
