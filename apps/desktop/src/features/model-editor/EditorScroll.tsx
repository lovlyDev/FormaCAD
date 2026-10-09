import { useLayoutEffect,useRef,type ReactNode } from "react";
import { useEditorPreference } from "./editorPreferences";
export function EditorScroll({children}:{children:ReactNode}){
 const [saved,setSaved]=useEditorPreference("scroll.top",0);const ref=useRef<HTMLDivElement>(null),timer=useRef<ReturnType<typeof setTimeout>|null>(null),latest=useRef(saved),save=useRef(setSaved);save.current=setSaved;
 useLayoutEffect(()=>{
  if(ref.current)ref.current.scrollTop=Math.max(0,Number.isFinite(saved)?saved:0);
  return()=>{if(timer.current)clearTimeout(timer.current);save.current(latest.current);};
 },[]);
 return <div ref={ref} className="model-editor-scroll" onScroll={event=>{latest.current=event.currentTarget.scrollTop;if(timer.current)clearTimeout(timer.current);timer.current=setTimeout(()=>save.current(latest.current),150);}}>{children}</div>;
}
