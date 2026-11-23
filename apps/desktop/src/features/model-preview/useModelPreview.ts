import { useEffect,useRef,useState } from "react";
import { disposeModel } from "../../lib/model";
import { sourceRefFor,type ProjectSourceBinding } from "../project-sources/sourceBinding";
import { cancelModelPreview,previewModel,modelPreviewAvailable,type ModelPreview } from "./previewApi";
/** Serial requests; an opening owns its cancellation and retained preview geometry. */
export function useModelPreview(projectId:string,source:string,revision:string|null,enabled:boolean,refresh:number,binding?:ProjectSourceBinding|null){
 const valid=binding===undefined||!!binding&&binding.projectId===projectId;
 const folder=valid&&binding?sourceRefFor(binding,projectId):undefined;
 const owner=JSON.stringify([projectId,binding?.openingId,folder?.sourceId,folder?.taskScopeId,folder?.commitId,valid]);
 const key=JSON.stringify({owner,source,revision,refresh}),available=modelPreviewAvailable()&&valid;
 const latest=useRef(key);latest.current=key;const wanted=useRef(enabled);wanted.current=enabled;
 const flight=useRef(false),alive=useRef(true),owned=useRef<{owner:string;preview:ModelPreview}|null>(null);
 const [tick,setTick]=useState(0),[pending,setPending]=useState(false),[state,setState]=useState<{key:string;preview:ModelPreview|null;error:string|null}>({key:"",preview:null,error:null});
 useEffect(()=>{
  alive.current=true;
  return()=>{
   alive.current=false;
   if(available)void (folder?cancelModelPreview(projectId,folder):cancelModelPreview(projectId)).catch(()=>{});
   if(owned.current?.owner===owner){disposeModel(owned.current.preview.object);owned.current=null;}
  };
 },[owner,available]);
 useEffect(()=>{
  if(!enabled||!available||!projectId){
   if(flight.current&&available)void (folder?cancelModelPreview(projectId,folder):cancelModelPreview(projectId)).catch(()=>{});
   if(owned.current?.owner===owner){disposeModel(owned.current.preview.object);owned.current=null;setState({key:"",preview:null,error:null});}return;
  }
  if(flight.current||state.key===key)return;
  const timer=setTimeout(()=>{
   flight.current=true;setPending(true);
   const mayAdopt=()=>alive.current&&wanted.current&&latest.current===key;
   const request=folder?previewModel(projectId,source,revision,mayAdopt,folder):previewModel(projectId,source,revision,mayAdopt);
   void request.then(preview=>{
    if(!mayAdopt()){disposeModel(preview.object);return;}
    if(owned.current)disposeModel(owned.current.preview.object);
    owned.current={owner,preview};setState({key,preview,error:null});
   },error=>{if(mayAdopt())setState({key,preview:null,error:String(error)});}).finally(()=>{
    flight.current=false;if(alive.current){setPending(false);setTick(value=>value+1);}
   });
  },500);
  return()=>clearTimeout(timer);
 },[key,enabled,available,projectId,tick,state.key]);
 const fresh=enabled&&state.key===key;
 return {available,pending,waiting:enabled&&available&&!fresh,preview:enabled&&available?(fresh?state.preview:owned.current?.owner===owner?owned.current.preview:null):null,error:fresh?state.error:null};
}
