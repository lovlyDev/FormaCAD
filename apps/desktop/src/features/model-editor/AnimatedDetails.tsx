import { Children,isValidElement,useLayoutEffect,useRef,useState,type ComponentProps,type MouseEvent } from "react";
import "./EditorDisclosure.css";
type Props=Omit<ComponentProps<"details">,"open"|"onToggle">&{open:boolean;onOpenChange:(open:boolean)=>void};
/** Intercept native toggling: animate both ways before hiding the content. */
export function AnimatedDetails({open,onOpenChange,children,...props}:Props){
 const [present,setPresent]=useState(open);
 const element=useRef<HTMLDivElement>(null),animation=useRef<Animation|null>(null),previous=useRef(false);
 useLayoutEffect(()=>()=>{animation.current?.cancel();animation.current=null;},[]);
 useLayoutEffect(()=>{
  const body=element.current;if(!body)return;
  const from=previous.current||present?body.getBoundingClientRect().height:0;
  const opacity=previous.current||present?Number(getComputedStyle(body).opacity):0;
  previous.current=open;animation.current?.cancel();animation.current=null;
  const reduced=window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  if(reduced||!body.animate){setPresent(open);return;}
  if(!open&&!present)return;
  if(open)setPresent(true);
  body.dataset.animationCount=String(Number(body.dataset.animationCount??0)+1);
  const active=body.animate([{height:`${from}px`,opacity},{height:open?`${body.scrollHeight}px`:"0px",opacity:open?1:0}],{duration:260,easing:"cubic-bezier(.2,.8,.2,1)",fill:"both"});
  animation.current=active;
  active.onfinish=()=>{if(animation.current!==active)return;setPresent(open);active.cancel();animation.current=null;};
 // Presence is intentionally held through close and must not restart its animation.
 },[open]);
 const toggle=(event:MouseEvent<HTMLElement>)=>{if((event.target as HTMLElement).closest("summary")?.parentElement!==event.currentTarget)return;event.preventDefault();onOpenChange(!open);};
 const parts=Children.toArray(children),summary=parts.filter(child=>isValidElement(child)&&child.type==="summary"),body=parts.filter(child=>!isValidElement(child)||child.type!=="summary");
 return <details {...props} open={open||present} data-expanded={open} onClick={toggle} onToggle={event=>{if(!animation.current&&event.currentTarget.open!==open)onOpenChange(event.currentTarget.open);}}>
 {summary}<div ref={element} className="editor-disclosure-body" aria-hidden={!open} inert={!open} data-expanded={open} onPointerDown={()=>animation.current?.finish()}>{body}</div></details>;
}
