import { describe,it,expect } from "vitest";
import { readPairReport } from "./pairSchema";

// Independent wire boundaries rather than kernel computation tests.
const face={schemaVersion:1,kind:"face",ownerFeatureId:"pad",role:"box-face:xmin",occurrencePath:[]};
const second={...face,role:"box-face:xmax"};
function distance(){return{projectId:"project",schemaVersion:1,requestId:"00000000-0000-4000-8000-000000000001",engine:{name:"occt",protocolVersion:1},evaluationSource:"rebuiltAuthoredBody",revisionId:"00000000-0000-4000-8000-000000000002",bodyId:"body",documentSha256:"a".repeat(64),sourceSha256:"b".repeat(64),sourceSize:128,importedAssetSeals:[],query:{kind:"minimumDistance",first:face,second},result:{kind:"minimumDistance",distanceMm:40,pointAMm:[0,2,3],pointBMm:[40,2,3]}};}
describe("pair wire boundary",()=>{
 it("accepts touching zero and rejects fabricated inconsistent witnesses",()=>{
  const report=distance();report.result.distanceMm=0;report.result.pointBMm=[0,2,3];
  expect(readPairReport(report).result.kind).toBe("minimumDistance");
  report.result.pointBMm=[1,2,3];expect(()=>readPairReport(report)).toThrow("MEASUREMENT_REPORT_INVALID");
 });
 it("rejects scalar/result variants and unknown caller geometry fields",()=>{
  const report=distance();expect(()=>readPairReport({...report,query:{...report.query,point:[0,0,0]}})).toThrow();
  expect(()=>readPairReport({...report,result:{kind:"faceNormalAngle",angleDeg:90}})).toThrow();
  expect(()=>readPairReport({...report,result:{...report.result,distanceMm:-40}})).toThrow();
 });
 it("distinguishes signed outward face-normal and acute straight-edge ranges",()=>{
  const report=distance();
  const normal={...report,query:{kind:"faceNormalAngle",first:face,second},result:{kind:"faceNormalAngle",angleDeg:180}};
  expect(readPairReport(normal).result.kind).toBe("faceNormalAngle");
  expect(()=>readPairReport({...normal,result:{kind:"faceNormalAngle",angleDeg:181}})).toThrow();
  const edge={schemaVersion:1,kind:"edge",ownerFeatureId:"pad",role:"box-edge:x:ymin:zmin",occurrencePath:[]};
  const acute={...report,query:{kind:"edgeAcuteAngle",first:edge,second:edge},result:{kind:"edgeAcuteAngle",angleDeg:90}};
  expect(readPairReport(acute).result.kind).toBe("edgeAcuteAngle");
  expect(()=>readPairReport({...acute,result:{kind:"edgeAcuteAngle",angleDeg:91}})).toThrow();
 });
});
