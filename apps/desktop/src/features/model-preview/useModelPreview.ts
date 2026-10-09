import { useEffect,useRef,useState } from "react";
import { disposeModel } from "../../lib/model";
import { cancelModelPreview,previewModel,modelPreviewAvailable,type ModelPreview } from "./previewApi";
/** At most one request per editor; debounce the latest source while an older request finishes. */
export function useModelPreview(projectId:string,source:string,revision:string|null,enabled:boolean,refresh:number){
 const key=JSON.stringify({projectId,source,revision,refresh}),available=modelPreviewAvailable();
 const latest=useRef(key);latest.current=key;const wanted=useRef(enabled);wanted.current=enabled;
 const flight=useRef(false),alive=useRef(true),owned=useRef<ModelPreview|null>(null);
 const [tick,setTick]=useState(0),[pending,setPending]=useState(false),[state,setState]=useState<{key:string;preview:ModelPreview|null;error:string|null}>({key:"",preview:null,error:null});
 useEffect(()=>{alive.current=true;return()=>{alive.current=false;if(available)void cancelModelPreview(projectId).catch(()=>{});if(owned.current){disposeModel(owned.current.object);owned.current=null;}};},[projectId,available]);
 useEffect(()=>{
  if(!enabled||!available||!projectId){if(flight.current&&available)void cancelModelPreview(projectId).catch(()=>{});if(owned.current){disposeModel(owned.current.object);owned.current=null;setState({key:"",preview:null,error:null});}return;}
  if(flight.current||state.key===key)return;
  const timer=setTimeout(()=>{
   flight.current=true;setPending(true);const request=JSON.parse(key) as {projectId:string;source:string;revision:string|null};
   void previewModel(request.projectId,request.source,request.revision,()=>alive.current&&wanted.current&&latest.current===key).then(preview=>{
    if(!alive.current||latest.current!==key||!wanted.current){disposeModel(preview.object);return;}
    if(owned.current)disposeModel(owned.current.object);owned.current=preview;setState({key,preview,error:null});
   },error=>{if(alive.current&&latest.current===key&&wanted.current)setState({key,preview:null,error:String(error)});}).finally(()=>{
    flight.current=false;if(alive.current){setPending(false);setTick(value=>value+1);}
   });
  },500);
  return()=>clearTimeout(timer);
 },[key,enabled,available,projectId,tick,state.key]);
 const fresh=enabled&&state.key===key;
 return {available,pending,waiting:enabled&&available&&!fresh,preview:enabled&&available?(fresh?state.preview:owned.current):null,error:fresh?state.error:null};
}
