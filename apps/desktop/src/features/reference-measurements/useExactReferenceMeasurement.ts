import { useEffect,useMemo,useRef,useState,useSyncExternalStore } from "react";
import { captureMeasurement,type MeasurementContext } from "./measurementCapture";
import { MeasurementSession } from "./measurementSession";
import { measureReference } from "./measurementApi";

export function useExactReferenceMeasurement(context:MeasurementContext) {
  const [session]=useState(()=>new MeasurementSession());
  const captured=useMemo(()=>captureMeasurement(context),[context.project,context.bodyId,context.query,context.selectionToken,context.interactive]);
  const snapshot=useSyncExternalStore(session.subscribe,session.snapshot,session.snapshot);
  useEffect(()=>{session.setContext(captured);},[captured,session]);
  const mounted=useRef(false);
  useEffect(()=>{
    mounted.current=true;
    return()=>{mounted.current=false;queueMicrotask(()=>{if(!mounted.current)session.close();});};
  },[session]);
  // Hides obsolete numbers on the render that changes selection, before effects.
  const aligned=!!captured&&snapshot.key===captured.key;
  return {available:!!captured,working:snapshot.working,phase:aligned?snapshot.phase:"idle" as const,report:aligned?snapshot.report:null,error:aligned?snapshot.error:undefined,
    request:()=>{session.setContext(captured);return session.run(measureReference);},stopWaiting:()=>session.stopWaiting()};
}
