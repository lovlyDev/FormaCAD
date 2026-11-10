import { useCallback,useState } from "react";
import { Canvas,type CanvasProps } from "@react-three/fiber";
/**
 * R3F configures its renderer asynchronously. Its internal div ref can already
 * be null when onCreated runs after a fast unmount. Keep an actual owned node
 * as the event target; never pass a mutable ref into that late callback.
 */
export function StableCanvas({children,...props}:Omit<CanvasProps,"eventSource">){
 const [source,setSource]=useState<HTMLDivElement|null>(null);
 const attach=useCallback((node:HTMLDivElement|null)=>{if(node)setSource(node);},[]);
 return <div ref={attach} className="canvas-event-host" style={{position:"relative",width:"100%",height:"100%"}}>
  {source&&<Canvas {...props} eventSource={source}>{children}</Canvas>}
 </div>;
}
