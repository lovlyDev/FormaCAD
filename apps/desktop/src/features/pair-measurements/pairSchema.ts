import { z } from "zod";
import { topologyReferenceSchema } from "../viewer/topology/topologyReference";
const id=z.string().regex(/^[A-Za-z0-9_-]{1,80}$/),sha=z.string().regex(/^[a-f0-9]{64}$/);
const vector=z.tuple([z.number().finite().min(-1e9).max(1e9),z.number().finite().min(-1e9).max(1e9),z.number().finite().min(-1e9).max(1e9)]);
const face=topologyReferenceSchema.refine(ref=>ref.kind==="face"),edge=topologyReferenceSchema.refine(ref=>ref.kind==="edge");
export const pairQuerySchema=z.discriminatedUnion("kind",[
 z.strictObject({kind:z.literal("minimumDistance"),first:topologyReferenceSchema,second:topologyReferenceSchema}),
 z.strictObject({kind:z.literal("faceNormalAngle"),first:face,second:face}),
 z.strictObject({kind:z.literal("edgeAcuteAngle"),first:edge,second:edge}),
]);
export type PairQuery=z.infer<typeof pairQuerySchema>;
export type PairKind=PairQuery["kind"];
const result=z.discriminatedUnion("kind",[
 z.strictObject({kind:z.literal("minimumDistance"),distanceMm:z.number().finite().nonnegative(),pointAMm:vector,pointBMm:vector}),
 z.strictObject({kind:z.literal("faceNormalAngle"),angleDeg:z.number().finite().min(0).max(180)}),
 z.strictObject({kind:z.literal("edgeAcuteAngle"),angleDeg:z.number().finite().min(0).max(90)}),
]);
const seal=z.strictObject({assetId:z.string().regex(/^step_[a-f0-9]{64}$/),sha256:sha,size:z.number().int().positive().max(40*1024*1024)}).refine(value=>value.assetId===`step_${value.sha256}`);
export const pairReportSchema=z.strictObject({
 projectId:id,schemaVersion:z.literal(1),requestId:z.string().uuid(),engine:z.strictObject({name:z.literal("occt"),protocolVersion:z.literal(1)}),evaluationSource:z.literal("rebuiltAuthoredBody"),
 revisionId:z.string().uuid(),bodyId:id,documentSha256:sha,sourceSha256:sha,sourceSize:z.number().int().positive().max(40*1024*1024),
 importedAssetSeals:z.array(seal).max(32).refine(items=>items.reduce((sum,value)=>sum+value.size,0)<=128*1024*1024&&items.every((value,index)=>!index||items[index-1].assetId<value.assetId)),query:pairQuerySchema,result,
}).refine(report=>{
 if(report.query.kind!==report.result.kind)return false;
 if(report.result.kind!=="minimumDistance")return true;
 const measured=report.result;
 const separation=Math.hypot(...measured.pointAMm.map((value,index)=>value-measured.pointBMm[index]));
 return Number.isFinite(separation)&&Math.abs(separation-measured.distanceMm)<=Math.max(1e-6,measured.distanceMm*1e-9);
});
export type PairReport=z.infer<typeof pairReportSchema>;
export function readPairReport(value:unknown):PairReport {
 let encoded:string;try{encoded=JSON.stringify(value);}catch{throw Error("MEASUREMENT_REPORT_INVALID");}
 if(typeof encoded!=="string"||new TextEncoder().encode(encoded).byteLength>64*1024)throw Error("MEASUREMENT_REPORT_INVALID");
 const parsed=pairReportSchema.safeParse(value);if(!parsed.success)throw Error("MEASUREMENT_REPORT_INVALID");return parsed.data;
}
