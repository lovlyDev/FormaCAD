import { useEffect,useMemo,useRef,useState } from "react";
import type { Project } from "../../../../types";
import type { FaceSelection } from "../../faceSelection";
import { captureFaceSection } from "./faceSectionCapture";
import { sectionReference } from "./faceSectionApi";
import type { FaceSectionReport } from "./faceSectionSchema";

/** One unfinished invocation survives context changes; only a current verified result clips. */
export function useFaceSection(project:Project|null,selection:FaceSelection|null,offsetMm:number,active:boolean,sceneToken:string,automatic=true,blocked=false){
 const capture=useMemo(()=>{const value=captureFaceSection(project,selection,offsetMm,active);return value?{...value,key:JSON.stringify([value.key,sceneToken])}:null;},[project,selection,offsetMm,active,sceneToken]);
 const forced=useRef<{key:string;epoch:number}|null>(null);
 const latest=useRef({key:"",epoch:0});
 if(latest.current.key!==(capture?.key??"")){latest.current={key:capture?.key??"",epoch:latest.current.epoch+1};forced.current=null;}
 const mounted=useRef(false),inFlight=useRef(false);
 const [forceTick,setForceTick]=useState(0);
 const [tick,setTick]=useState(0),[working,setWorking]=useState(false),[result,setResult]=useState<{key:string;report?:FaceSectionReport;error?:unknown}>({key:""});
 useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;};},[]);
 useEffect(()=>{
  const forceCurrent=!!capture&&forced.current?.key===capture.key&&forced.current.epoch===latest.current.epoch;
  if(!capture||blocked||inFlight.current||(!automatic&&!forceCurrent)||(result.key===capture.key&&!forceCurrent))return;
  const delay=forceCurrent?0:400;
  const timer=setTimeout(()=>{
   const epoch=latest.current.epoch;
   forced.current=null;inFlight.current=true;setWorking(true);
   void sectionReference(capture).then(report=>{if(mounted.current&&latest.current.key===capture.key&&latest.current.epoch===epoch)setResult({key:capture.key,report});},error=>{if(mounted.current&&latest.current.key===capture.key&&latest.current.epoch===epoch)setResult({key:capture.key,error});}).finally(()=>{inFlight.current=false;if(mounted.current){setWorking(false);setTick(value=>value+1);}});
  },delay);return()=>clearTimeout(timer);
 },[capture,result.key,tick,forceTick,automatic,blocked]);
 const aligned=!!capture&&result.key===capture.key,report=aligned?result.report:undefined;
 return{available:!!capture,working:working||blocked,pending:!!capture&&(working||(automatic&&!aligned)),report,error:aligned?result.error:undefined,plane:report?.geometry.plane,calculate:()=>{if(!capture||blocked||inFlight.current)return;forced.current={key:capture.key,epoch:latest.current.epoch};setForceTick(value=>value+1);}};
}
