import { describe,it,expect } from "vitest";
import { PairSession } from "./pairSession";
import { readPairReport } from "./pairSchema";
import type { PairCapture } from "./pairCapture";

// Raw reference field insertion order intentionally differs from the canonical parsed wire report.
const first={schemaVersion:1 as const,kind:"face" as const,ownerFeatureId:"pad",role:"box-face:xmin" as const,occurrencePath:[]};
const second={...first,role:"box-face:xmax" as const};
const capture:PairCapture={key:"context-a",request:{projectId:"project",expectedRevision:"00000000-0000-4000-8000-000000000002",bodyId:"body",query:{kind:"minimumDistance",first,second}},rawProgram:"{}",sourceSha256:"b".repeat(64),sourceSize:128};
const report=()=>readPairReport({projectId:"project",schemaVersion:1,requestId:"00000000-0000-4000-8000-000000000001",engine:{name:"occt",protocolVersion:1},evaluationSource:"rebuiltAuthoredBody",revisionId:capture.request.expectedRevision,bodyId:"body",documentSha256:"a".repeat(64),sourceSha256:capture.sourceSha256,sourceSize:128,importedAssetSeals:[],query:capture.request.query,result:{kind:"minimumDistance",distanceMm:40,pointAMm:[0,0,0],pointBMm:[40,0,0]}});
describe("pair request ownership",()=>{
 it("keeps one unfinished request and rejects a result after context replacement",async()=>{
  const session=new PairSession();session.setContext(capture);
  let finish!:(value:ReturnType<typeof report>)=>void;
  const pending=session.run(()=>new Promise(resolve=>{finish=resolve;}));
  let anotherCalled=false;
  await session.run(async()=>{anotherCalled=true;return report();});expect(anotherCalled).toBe(false);
  session.setContext({...capture,key:"context-b"});expect(session.snapshot().report).toBeNull();expect(session.snapshot().working).toBe(true);
  finish(report());await pending;expect(session.snapshot().report).toBeNull();expect(session.snapshot().working).toBe(false);
 });
 it("stop waiting prevents adoption while a failed retry keeps the same confirmed pair",async()=>{
  const session=new PairSession();session.setContext(capture);await session.run(async()=>report());
  await session.run(async()=>{throw Error("MEASUREMENT_FAILED");});expect(session.snapshot().phase).toBe("failed");expect(session.snapshot().report?.result.kind).toBe("minimumDistance");
  let finish!:(value:ReturnType<typeof report>)=>void;
  const pending=session.run(()=>new Promise(resolve=>{finish=resolve;}));session.stopWaiting();
  expect(session.snapshot().working).toBe(true);finish(report());await pending;expect(session.snapshot().phase).toBe("ready");
 });
});
