import { AlertTriangle,Check,PenLine } from "lucide-react";
import { t } from "../../../i18n";
import { readTypedCadDocument } from "../../../lib/typedCadDocument";
import { readSketchOperation } from "../../../lib/sketchDocument";
import type { SketchReviewItem } from "./useSketchBuildReview";
import "./SketchBuildReview.css";
export function SketchBuildReview({source,items,pending,error,disabled,onChange}:{source:string;items:SketchReviewItem[];pending:boolean;error:boolean;disabled:boolean;onChange:(source:string)=>void}){
 if(!pending&&!error&&!items.length)return null;
 const change=(item:SketchReviewItem,drawn:boolean)=>{const doc=readTypedCadDocument(source);if(!doc)return;onChange(JSON.stringify({...doc,features:doc.features.map(feature=>{if(feature.id!==item.id)return feature;const operation=readSketchOperation(feature.operation);return operation?{...feature,operation:{...operation,...(drawn?{constraints:[]}:{points:item.analysis.solvedPoints})}}:feature;})},null,2));};
 return <aside className="sketch-build-review" role="status"><strong><AlertTriangle size={16}/>{t("Check sketch shape before building")}</strong>
 {pending?<p>{t("Checking sketch constraints…")}</p>:error?<p>{t("Sketch analysis failed")}</p>:items.map(item=><div key={item.id}><b>{item.name}</b><p>{item.analysis.status==="solved"?t("Constraints change the drawn profile. Choose which shape to build."):(item.analysis.status==="conflict"?t("Sketch constraints conflict"):t("Sketch profile is invalid"))}</p>
 {item.analysis.status==="solved"&&<div className="sketch-review-actions"><button type="button" disabled={disabled} onClick={()=>change(item,true)}><PenLine size={14}/>{t("Use drawn profile and remove constraints")}</button><button type="button" disabled={disabled} onClick={()=>change(item,false)}><Check size={14}/>{t("Use solved coordinates")}</button></div>}</div>)}
 </aside>;
}
