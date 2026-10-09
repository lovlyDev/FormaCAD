import { expect,it } from "vitest";
import { MeasurementSession } from "./measurementSession";
import type { MeasurementCapture } from "./measurementCapture";
import type { MeasurementReport } from "./measurementSchema";
const revision="c661482b-378a-4fa3-a426-ad9bf6621478",sha="a".repeat(64);
function capture(key="A"):MeasurementCapture{return{key,request:{projectId:key,expectedRevision:revision,bodyId:"body",query:{kind:"bodyMetrics"}},sourceSha256:sha,sourceSize:100,rawProgram:"{}",selectionToken:"body"};}
function report(context:MeasurementCapture):MeasurementReport{return{projectId:context.request.projectId,schemaVersion:1,requestId:"0fdca3bb-60ac-4014-99e9-28b8306687c7",engine:{name:"occt",protocolVersion:1},evaluationSource:"rebuiltAuthoredBody",revisionId:revision,bodyId:"body",sourceSha256:sha,sourceSize:100,documentSha256:sha,importedAssetSeals:[],query:{kind:"bodyMetrics"},result:{kind:"bodyMetrics",volumeMm3:8000,areaMm2:2800,faceCount:6,edgeCount:12,extentsMm:[40,20,10]}};}
it("rejects late project, head or selection responses and local stop-waiting results",async()=>{
  const session=new MeasurementSession(),base=capture();session.setContext(base);
  let finish!: (report:MeasurementReport)=>void;
  const pending=session.run(()=>new Promise(resolve=>{finish=resolve;}));
  session.setContext({...capture("B"),selectionToken:"different-edge"});finish(report(base));await pending;
  expect(session.snapshot().report).toBeNull();expect(session.snapshot().key).toBe("B");
  session.setContext(base);const stopped=session.run(()=>new Promise(resolve=>{finish=resolve;}));
  session.stopWaiting();finish(report(base));await stopped;expect(session.snapshot().report).toBeNull();
});
it("local stop-waiting cannot enqueue another native job before the first finishes",async()=>{
  const session=new MeasurementSession(),base=capture();session.setContext(base);let finish!:(report:MeasurementReport)=>void,calls=0;
  const running=session.run(()=>{calls++;return new Promise(resolve=>{finish=resolve;});});
  session.stopWaiting();session.setContext(capture("B"));
  await session.run(async captured=>{calls++;return report(captured);});
  expect(calls).toBe(1);expect(session.snapshot().working).toBe(true);
  finish(report(base));await running;expect(session.snapshot().working).toBe(false);expect(session.snapshot().report).toBeNull();
});
it("retains only a proven same-context result on failure, then removes it on source change",async()=>{
  const session=new MeasurementSession(),base=capture();session.setContext(base);
  await session.run(async()=>report(base));const previous=session.snapshot().report;
  await session.run(async()=>{throw Error("worker unavailable");});
  expect(session.snapshot().phase).toBe("failed");expect(session.snapshot().report).toBe(previous);
  session.setContext({...base,key:"same-head-source-edited",rawProgram:"changed"});
  expect(session.snapshot().report).toBeNull();
});
