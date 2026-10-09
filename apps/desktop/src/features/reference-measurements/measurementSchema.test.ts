import { expect,it } from "vitest";
import { measurementQuerySchema,measurementReportSchema } from "./measurementSchema";
const edge={schemaVersion:1,kind:"edge",ownerFeatureId:"pad",role:"box-edge:x:ymin:zmin",occurrencePath:["rotate"]};
it("rejects mesh ordinal substitution, mixed reference kinds and guessed circle fields",()=>{
  expect(measurementQuerySchema.safeParse({kind:"edgeLength",reference:edge}).success).toBe(true);
  expect(measurementQuerySchema.safeParse({kind:"edgeLength",edgeOrdinal:4}).success).toBe(false);
  expect(measurementQuerySchema.safeParse({kind:"faceArea",reference:edge}).success).toBe(false);
  expect(measurementQuerySchema.safeParse({kind:"edgeRadius",reference:edge,radiusMm:2}).success).toBe(false);
  expect(measurementQuerySchema.safeParse({kind:"bodyMetrics",reference:edge}).success).toBe(false);
});
it("normalizes native reference property order without weakening its authored identity",()=>{
  const native={kind:"edgeLength",reference:{kind:"edge",occurrencePath:["rotate"],ownerFeatureId:"pad",role:"box-edge:x:ymin:zmin",schemaVersion:1}};
  expect(JSON.stringify(measurementQuerySchema.parse(native))).toBe(JSON.stringify(measurementQuerySchema.parse({kind:"edgeLength",reference:edge})));
  expect(measurementQuerySchema.parse({...native,reference:{...native.reference,occurrencePath:["other_branch"]}})).not.toEqual(measurementQuerySchema.parse(native));
});
it("rejects wrong result kinds and non-unit outward normals in a native report",()=>{
  const report={projectId:"A",schemaVersion:1,requestId:"0fdca3bb-60ac-4014-99e9-28b8306687c7",engine:{name:"occt",protocolVersion:1},evaluationSource:"rebuiltAuthoredBody",revisionId:"c661482b-378a-4fa3-a426-ad9bf6621478",bodyId:"body",sourceSha256:"a".repeat(64),sourceSize:100,documentSha256:"b".repeat(64),importedAssetSeals:[],query:{kind:"edgeLength",reference:edge},result:{kind:"edgeRadius",radiusMm:2}};
  expect(measurementReportSchema.safeParse(report).success).toBe(false);
  expect(measurementReportSchema.safeParse({...report,query:{kind:"planarFace",reference:{...edge,kind:"face",role:"box-face:zmax"}},result:{kind:"planarFace",areaMm2:100,originMm:[0,0,10],normal:[0,0,2]}}).success).toBe(false);
});
it("accepts a bound native diameter and rejects forged values and result-kind substitution",()=>{
  const reference={...edge,role:"cylinder-edge:top"};
  const query={kind:"edgeDiameter",reference};
  expect(measurementQuerySchema.safeParse(query).success).toBe(true);
  expect(measurementQuerySchema.safeParse({...query,diameterMm:20}).success).toBe(false);
  const report={projectId:"A",schemaVersion:1,requestId:"0fdca3bb-60ac-4014-99e9-28b8306687c7",engine:{name:"occt",protocolVersion:1},evaluationSource:"rebuiltAuthoredBody",revisionId:"c661482b-378a-4fa3-a426-ad9bf6621478",bodyId:"body",sourceSha256:"a".repeat(64),sourceSize:100,documentSha256:"b".repeat(64),importedAssetSeals:[],query,result:{kind:"edgeDiameter",diameterMm:20}};
  expect(measurementReportSchema.safeParse(report).success).toBe(true);
  for(const value of [0,-2,NaN,Infinity])expect(measurementReportSchema.safeParse({...report,result:{kind:"edgeDiameter",diameterMm:value}}).success).toBe(false);
  expect(measurementReportSchema.safeParse({...report,result:{kind:"edgeRadius",radiusMm:10}}).success).toBe(false);
});
