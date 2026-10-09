import { z } from "zod";
import { topologyReferenceSchema } from "../viewer/topology/topologyReference";

const id=z.string().regex(/^[A-Za-z0-9_-]{1,80}$/),digest=z.string().regex(/^[a-f0-9]{64}$/);
const positive=z.number().finite().positive(),coordinate=z.number().finite().min(-1e9).max(1e9);
const vector=z.tuple([coordinate,coordinate,coordinate]);
const edge=topologyReferenceSchema.refine(reference=>reference.kind==="edge"),face=topologyReferenceSchema.refine(reference=>reference.kind==="face");
export const measurementQuerySchema=z.discriminatedUnion("kind",[
  z.strictObject({kind:z.literal("bodyMetrics")}),
  z.strictObject({kind:z.literal("edgeLength"),reference:edge}),
  z.strictObject({kind:z.literal("edgeRadius"),reference:edge}),
  z.strictObject({kind:z.literal("edgeDiameter"),reference:edge}),
  z.strictObject({kind:z.literal("faceArea"),reference:face}),
  z.strictObject({kind:z.literal("planarFace"),reference:face}),
]);
export type MeasurementQuery=z.infer<typeof measurementQuerySchema>;
const count=z.number().int().positive().max(0xffffffff);
const resultSchema=z.discriminatedUnion("kind",[
  z.strictObject({kind:z.literal("bodyMetrics"),volumeMm3:positive,areaMm2:positive,faceCount:count,edgeCount:count,extentsMm:z.tuple([positive.max(1e9),positive.max(1e9),positive.max(1e9)])}),
  z.strictObject({kind:z.literal("edgeLength"),lengthMm:positive}),
  z.strictObject({kind:z.literal("edgeRadius"),radiusMm:positive}),
  z.strictObject({kind:z.literal("edgeDiameter"),diameterMm:positive}),
  z.strictObject({kind:z.literal("faceArea"),areaMm2:positive}),
  z.strictObject({kind:z.literal("planarFace"),areaMm2:positive,originMm:vector,normal:vector.refine(normal=>Math.abs(normal.reduce((sum,value)=>sum+value*value,0)-1)<=1e-8)}),
]);
const assetSeal=z.strictObject({assetId:z.string().regex(/^step_[a-f0-9]{64}$/),sha256:digest,size:z.number().int().positive().max(40*1024*1024)}).refine(seal=>seal.assetId===`step_${seal.sha256}`);
export const measurementReportSchema=z.strictObject({
  projectId:id,schemaVersion:z.literal(1),requestId:z.string().uuid(),engine:z.strictObject({name:z.literal("occt"),protocolVersion:z.literal(1)}),
  evaluationSource:z.literal("rebuiltAuthoredBody"),revisionId:z.string().uuid(),bodyId:id,sourceSha256:digest,sourceSize:z.number().int().positive().max(40*1024*1024),
  documentSha256:digest,importedAssetSeals:z.array(assetSeal).max(32).refine(seals=>seals.reduce((sum,seal)=>sum+seal.size,0)<=128*1024*1024&&seals.every((seal,index)=>!index||seals[index-1].assetId<seal.assetId)),
  query:measurementQuerySchema,result:resultSchema,
}).refine(report=>report.query.kind===report.result.kind);
export type MeasurementReport=z.infer<typeof measurementReportSchema>;
export type MeasurementValue=MeasurementReport["result"];

/** Bound malformed renderer payloads; host remains the authority for file seals and native values. */
export function readMeasurementReport(value:unknown):MeasurementReport {
  if(new TextEncoder().encode(JSON.stringify(value)).byteLength>64*1024)throw Error("MEASUREMENT_REPORT_INVALID");
  const parsed=measurementReportSchema.safeParse(value);
  if(!parsed.success)throw Error("MEASUREMENT_REPORT_INVALID");
  return parsed.data;
}
