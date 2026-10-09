import { resolveSketchBindings } from "../../../lib/sketchBindings";
import { useEffect,useMemo,useState } from "react";
import { readTypedCadDocument,activeFeatureIds } from "../../../lib/typedCadDocument";
import { readSketchOperation } from "../../../lib/sketchDocument";
import { analyzeSketch,sketchAnalysisAvailable,type SketchAnalysis } from "../../../lib/sketchAnalysis";
import { shapeDifference } from "./shapeDifference";
export type SketchReviewItem={id:string;name:string;analysis:SketchAnalysis};
export function useSketchBuildReview(source:string,enabled:boolean){
 const input=useMemo(()=>{const doc=readTypedCadDocument(source);if(!doc)return null;const active=activeFeatureIds(doc);return {parameters:doc.parameters,sketches:doc.features.filter(feature=>active.has(feature.id)).flatMap(feature=>{const operation=readSketchOperation(feature.operation);return operation&&(operation.constraints.length||operation.bindings?.length)?[{id:feature.id,name:feature.name,operation}]:[];})};},[source]);
 const available=enabled&&sketchAnalysisAvailable()&&!!input?.sketches.length;
 const [state,setState]=useState<{key:string;items:SketchReviewItem[];error:boolean}>({key:"",items:[],error:false});
 useEffect(()=>{if(!available||!input)return;let active=true;const timer=setTimeout(()=>{void(async()=>{const items:SketchReviewItem[]=[];for(const sketch of input.sketches){if(!active)return;const analysis=await analyzeSketch(sketch.operation,input.parameters,sketch.id);if(analysis.status!=="solved"||shapeDifference(resolveSketchBindings(sketch.operation,input.parameters).points,analysis.solvedPoints))items.push({id:sketch.id,name:sketch.name,analysis});}if(active)setState({key:source,items,error:false});})().catch(()=>{if(active)setState({key:source,items:[],error:true});});},250);return()=>{active=false;clearTimeout(timer);};},[source,available,input]);
 const fresh=state.key===source;return {pending:available&&!fresh,items:available&&fresh?state.items:[],error:available&&fresh&&state.error,blocked:available&&fresh&&(state.error||state.items.length>0)};
}
