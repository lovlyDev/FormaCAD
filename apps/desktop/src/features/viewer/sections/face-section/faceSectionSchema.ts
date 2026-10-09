import { z } from "zod";
import { topologyReferenceSchema } from "../../topology/topologyReference";
import { validFaceSectionGeometry } from "./faceSectionGeometry";
const coordinate=z.number().finite().min(-1e9).max(1e9),vector=z.tuple([coordinate,coordinate,coordinate]);
const normal=vector.refine(value=>Math.abs(value.reduce((sum,n)=>sum+n*n,0)-1)<=1e-8),sha=z.string().regex(/^[a-f0-9]{64}$/),id=z.string().regex(/^[A-Za-z0-9_-]{1,80}$/),planeCoordinate=z.number().finite().min(-10000).max(10000);
export const faceSectionQuerySchema=z.strictObject({reference:topologyReferenceSchema.refine(ref=>ref.kind==="face"),offsetMm:z.number().finite().min(-10000).max(10000),deflectionMm:z.number().finite().min(0.001).max(1)});
const seal=z.strictObject({assetId:z.string().regex(/^step_[a-f0-9]{64}$/),sha256:sha,size:z.number().int().positive().max(40*1024*1024)}).refine(item=>item.assetId===`step_${item.sha256}`);
export const faceSectionReportSchema=z.strictObject({
 projectId:id,schemaVersion:z.literal(1),requestId:z.string().uuid(),engine:z.strictObject({name:z.literal("occt"),protocolVersion:z.literal(1)}),
 evaluationSource:z.literal("rebuiltAuthoredFaceAndSealedStep"),revisionId:z.string().uuid(),bodyId:id,documentSha256:sha,sourceSha256:sha,sourceSize:z.number().int().positive().max(40*1024*1024),
 importedAssetSeals:z.array(seal).max(32).refine(items=>items.reduce((sum,item)=>sum+item.size,0)<=128*1024*1024&&items.every((item,index)=>!index||items[index-1].assetId<item.assetId)),
 query:faceSectionQuerySchema,face:z.strictObject({areaMm2:z.number().finite().positive(),originMm:vector,normal}),
 geometry:z.strictObject({plane:z.strictObject({originMm:z.tuple([planeCoordinate,planeCoordinate,planeCoordinate]),normal,deflectionMm:z.number().finite().min(0.001).max(1)}),totalLengthMm:z.number().finite().nonnegative(),curves:z.array(z.strictObject({id:z.string().regex(/^section-edge-[1-9][0-9]*$/),lengthMm:z.number().finite().positive(),closed:z.boolean(),pointsMm:z.array(vector).min(2).max(100000)})).max(4096).refine(items=>items.reduce((sum,item)=>sum+item.pointsMm.length,0)<=100000)}),
}).refine(report=>validFaceSectionGeometry(report.geometry)&&report.geometry.plane.deflectionMm===report.query.deflectionMm&&report.geometry.plane.normal.every((value,index)=>Math.abs(value-report.face.normal[index])<=1e-9)&&report.geometry.plane.originMm.every((value,index)=>{const expected=report.face.originMm[index]+report.face.normal[index]*report.query.offsetMm;return Math.abs(value-expected)<=1e-9*Math.max(1,Math.abs(expected));}));
export type FaceSectionReport=z.infer<typeof faceSectionReportSchema>;
export function readFaceSectionReport(value:unknown){
 let encoded:string;try{encoded=JSON.stringify(value);}catch{throw Error("SECTION_REFERENCE_REPORT_INVALID");}
 if(typeof encoded!=="string"||new TextEncoder().encode(encoded).byteLength>8*1024*1024)throw Error("SECTION_REFERENCE_REPORT_INVALID");
 const parsed=faceSectionReportSchema.safeParse(value);if(!parsed.success)throw Error("SECTION_REFERENCE_REPORT_INVALID");return parsed.data;
}
