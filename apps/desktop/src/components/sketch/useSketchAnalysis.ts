import { useEffect, useState } from "react";
import { analyzeSketch, sketchAnalysisAvailable, type SketchAnalysis } from "../../lib/sketchAnalysis";
import type { SketchOperation } from "../../lib/sketchDocument";
import type { CadParameter } from "../../lib/cadParameters";
export function useSketchAnalysis(operation:SketchOperation,parameters:CadParameter[],featureId:string) {
  const key=JSON.stringify({operation,parameters,featureId});
  const available=sketchAnalysisAvailable();
  const [state,setState]=useState<{key:string;analysis:SketchAnalysis|null;error:boolean;pending:boolean}>({key:"",analysis:null,error:false,pending:false});
  useEffect(()=>{
    if(!available)return;
    let active=true;
    const timer=setTimeout(()=>{
      const request=JSON.parse(key) as {operation:SketchOperation;parameters:CadParameter[];featureId:string};
      setState({key,analysis:null,error:false,pending:true});
      void analyzeSketch(request.operation,request.parameters,request.featureId).then(
        (analysis)=>{if(active)setState({key,analysis,error:false,pending:false});},
        ()=>{if(active)setState({key,analysis:null,error:true,pending:false});},
      );
    },250);
    return ()=>{active=false;clearTimeout(timer);};
  },[key,available]);
  const fresh=state.key===key;
  return {available,analysis:fresh?state.analysis:null,error:fresh&&state.error,pending:available&&(!fresh||state.pending)};
}
