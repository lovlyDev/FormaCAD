import type { EdgeSelection } from "../viewer/edgeSelection";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import { t } from "../../i18n";
import { readTopologyReference } from "../viewer/topology/topologyReference";
import { matchesReferenceRoute } from "../viewer/topology/referenceRoute";
export function edgeFilletDraft(source:string,selection:EdgeSelection,revisionId:string|null,radiusMm:number):string|null{
 const document=readTypedCadDocument(source),body=document?.bodies.find(body=>body.id===selection.bodyId);
 if(!document||!body||!revisionId||selection.revisionId!==revisionId||!Number.isFinite(radiusMm)||radiusMm<=0||radiusMm>10000)return null;
 const reference=selection.topologyRef ? readTopologyReference(selection.topologyRef) : null;
 if(selection.topologyRef && (!reference || reference.kind!=="edge" || !reference.role.startsWith("box-edge:") || !matchesReferenceRoute(document,body.sourceFeatureId,reference)))return null;
 if(!reference&&(!selection.semanticKey||!/^box-edge:[xyz]:[xyz](min|max):[xyz](min|max)$/.test(selection.semanticKey)))return null;
 const id=`edge_fillet_${crypto.randomUUID().replace(/-/g,"").slice(0,12)}`;
 const operation=reference?{type:"filletReferencedEdge",bodyFeatureId:body.sourceFeatureId,reference,radius:{kind:"literal",mm:radiusMm}}:{type:"filletEdge",bodyFeatureId:body.sourceFeatureId,edgeKey:selection.semanticKey,radius:{kind:"literal",mm:radiusMm}};
 const next={...document,features:[...document.features,{id,name:t("Edge fillet"),operation}],bodies:document.bodies.map(item=>item.id===body.id?{...item,sourceFeatureId:id}:item)};
 const result=JSON.stringify(next,null,2);return readTypedCadDocument(result)?result:null;
}
