import { useEffect,useMemo,useRef,useState,useSyncExternalStore } from "react";
import type { TopologyReference } from "../viewer/topology/topologyReference";
import { capturePair,type PairScope } from "./pairCapture";
import type { PairKind } from "./pairSchema";
import { PairSession } from "./pairSession";
import { measurePair } from "./pairApi";
export function usePairMeasurement(scope:PairScope|null,first:TopologyReference|null,second:TopologyReference|null,kind:PairKind){
 const [session]=useState(()=>new PairSession());
 const capture=useMemo(()=>capturePair(scope,first,second,kind),[scope,first,second,kind]);
 const snapshot=useSyncExternalStore(session.subscribe,session.snapshot,session.snapshot);
 useEffect(()=>{session.setContext(capture);},[session,capture]);
 const mounted=useRef(false);
 useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;queueMicrotask(()=>{if(!mounted.current)session.close();});};},[session]);
 const aligned=!!capture&&snapshot.key===capture.key;
 return{available:!!capture,working:snapshot.working,phase:aligned?snapshot.phase:"idle" as const,report:aligned?snapshot.report:null,error:aligned?snapshot.error:undefined,
  request:()=>{session.setContext(capture);return session.run(measurePair);},stopWaiting:()=>session.stopWaiting()};
}
