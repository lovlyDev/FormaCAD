import { useEffect,useMemo,useState } from "react";
import type { Project } from "../../types";
import type { EdgeSelection } from "../viewer/edgeSelection";
import type { FaceSelection } from "../viewer/faceSelection";
import type { TopologyReference } from "../viewer/topology/topologyReference";
import { capturePairScope,currentPairReference } from "./pairCapture";
interface Slots {key:string;first:TopologyReference|null;second:TopologyReference|null}
export function usePairSlots(project:Project|null,bodyId:string|null,sceneToken:string,interactive:boolean,edge:EdgeSelection|null,face:FaceSelection|null){
 const scope=useMemo(()=>capturePairScope(project,bodyId,sceneToken,interactive),[project,bodyId,sceneToken,interactive]);
 const [slots,setSlots]=useState<Slots>({key:"",first:null,second:null});
 const key=scope?.key??"";
 // Aligned values disappear in the first render, before this housekeeping effect.
 const aligned=!!scope&&slots.key===key;
 useEffect(()=>{setSlots(value=>value.key===key?value:{key,first:null,second:null});},[key]);
 const current=useMemo(()=>currentPairReference(scope,edge,face),[scope,edge,face]);
 const capture=(slot:"first"|"second")=>{if(!scope||!current)return;setSlots(value=>({...(value.key===key?value:{key,first:null,second:null}),[slot]:current}));};
 return{scope,first:aligned?slots.first:null,second:aligned?slots.second:null,current,
  captureFirst:()=>capture("first"),captureSecond:()=>capture("second"),clear:()=>setSlots({key,first:null,second:null})};
}
