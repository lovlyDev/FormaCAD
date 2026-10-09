import { expect,it } from "vitest";
import { edgeFilletDraft } from "./edgeFilletDraft";
const source=JSON.stringify({schemaVersion:2,revisionId:"revision",parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle",width:{kind:"literal",mm:40},depth:{kind:"literal",mm:20}}},{id:"box",name:"Box",operation:{type:"extrude",sketchId:"profile",distance:{kind:"literal",mm:10}}}],bodies:[{id:"body_1",name:"Body",sourceFeatureId:"box"}]});
const edge={bodyId:"body_1",revisionId:"revision",edgeOrdinal:4,semanticKey:"box-edge:x:ymin:zmax"};
it("adds a draft modifier to the selected body without changing the saved source",()=>{
 const next=JSON.parse(edgeFilletDraft(source,edge,"revision",1)!);expect(next.features).toHaveLength(3);expect(next.features[2].operation).toEqual({type:"filletEdge",bodyFeatureId:"box",edgeKey:edge.semanticKey,radius:{kind:"literal",mm:1}});expect(next.bodies[0].sourceFeatureId).toBe(next.features[2].id);expect(JSON.parse(source).features).toHaveLength(2);
});
it("rejects stale selections, missing topology and invalid radii",()=>{
 expect(edgeFilletDraft(source,edge,"other",1)).toBeNull();expect(edgeFilletDraft(source,{...edge,bodyId:"absent"},"revision",1)).toBeNull();expect(edgeFilletDraft(source,{...edge,semanticKey:"unsupported"},"revision",1)).toBeNull();for(const radius of [0,-1,NaN,Infinity,10001])expect(edgeFilletDraft(source,edge,"revision",radius)).toBeNull();
});
it("authors a source-qualified fillet after rotation and rejects a reference from another branch",()=>{
 const document=JSON.parse(source);document.features.push({id:"turned",name:"Turned",operation:{type:"rotate",bodyFeatureId:"box",axisOriginMm:[0,0,0],axisDirection:[0,0,1],angleDeg:17}});document.bodies[0].sourceFeatureId="turned";
 const reference={schemaVersion:1 as const,kind:"edge" as const,ownerFeatureId:"box",role:"box-edge:x:ymin:zmax" as const,occurrencePath:["turned"]};
 const current=JSON.stringify(document),selected={...edge,semanticKey:undefined,topologyRef:reference};
 const operation=JSON.parse(edgeFilletDraft(current,selected,"revision",1)!).features.at(-1).operation;
 expect(operation.type).toBe("filletReferencedEdge");expect(operation.reference).toEqual(reference);expect(operation.bodyFeatureId).toBe("turned");expect(operation).not.toHaveProperty("edgeKey");
 expect(edgeFilletDraft(current,{...selected,topologyRef:{...reference,occurrencePath:["other_copy"]}},"revision",1)).toBeNull();
});
it("does not advertise unverified circular fillets when circular provenance is available",()=>{
 const document=JSON.parse(source);document.features[0].operation={type:"circle",radius:{kind:"literal",mm:10}};
 const selected={...edge,semanticKey:undefined,topologyRef:{schemaVersion:1 as const,kind:"edge" as const,ownerFeatureId:"box",role:"cylinder-edge:top" as const,occurrencePath:[]}};
 expect(edgeFilletDraft(JSON.stringify(document),selected,"revision",1)).toBeNull();
});
