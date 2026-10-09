import { StrictMode,type ReactNode } from "react";
import { act,renderHook,waitFor } from "@testing-library/react";
import { expect,it,vi } from "vitest";
import { useExactReferenceMeasurement } from "./useExactReferenceMeasurement";
import { MeasurementSession } from "./measurementSession";
import type { MeasurementContext,MeasurementCapture } from "./measurementCapture";
import type { MeasurementReport } from "./measurementSchema";
import { newProject } from "../../stores/workspace";
import { defaults } from "../../types";
const mocked=vi.hoisted(()=>({measure:vi.fn()}));
vi.mock("./measurementApi",()=>({measureReference:mocked.measure}));
const head="c661482b-378a-4fa3-a426-ad9bf6621478",sha="a".repeat(64);
function context():MeasurementContext {
  const project=newProject("Measured","blank","mm","codex");project.id="A";project.currentRevision=head;
  const program=JSON.stringify({schemaVersion:2,revisionId:head,parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle",width:{kind:"literal",mm:40},depth:{kind:"literal",mm:20}}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:{kind:"literal",mm:10}}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]});
  project.revisions=[{id:head,parent:null,createdAt:project.createdAt,prompt:"Source",parameters:defaults,program,source:"source.step"}];
  project.files=[{name:"source.step",kind:"model",sha256:sha,size:100}];
  return{project,bodyId:"body",query:{kind:"bodyMetrics"},selectionToken:"body",interactive:true};
}
function report(captured:MeasurementCapture):MeasurementReport {
  return{projectId:captured.request.projectId,schemaVersion:1,requestId:"0fdca3bb-60ac-4014-99e9-28b8306687c7",engine:{name:"occt",protocolVersion:1},evaluationSource:"rebuiltAuthoredBody",revisionId:captured.request.expectedRevision,bodyId:captured.request.bodyId,sourceSha256:sha,sourceSize:100,documentSha256:sha,importedAssetSeals:[],query:captured.request.query,result:{kind:"bodyMetrics",volumeMm3:8000,areaMm2:2800,faceCount:6,edgeCount:12,extentsMm:[40,20,10]}};
}
const strict=({children}:{children:ReactNode})=><StrictMode>{children}</StrictMode>;
it("StrictMode cleanup does not close the live measurement session or duplicate an explicit request",async()=>{
  mocked.measure.mockReset().mockImplementation(async captured=>report(captured));
  const view=renderHook(props=>useExactReferenceMeasurement(props),{initialProps:context(),wrapper:strict});
  await waitFor(()=>expect(view.result.current.available).toBe(true));
  await act(async()=>{await view.result.current.request();});
  expect(mocked.measure).toHaveBeenCalledTimes(1);expect(view.result.current.phase).toBe("ready");
  expect(view.result.current.report?.result.kind).toBe("bodyMetrics");
});
it.each(["project","head","selection","source"])("a delayed real hook response cannot adopt after %s changes",async changed=>{
  let finish!:(report:MeasurementReport)=>void,captured!:MeasurementCapture;
  mocked.measure.mockReset().mockImplementation(request=>{captured=request;return new Promise(resolve=>{finish=resolve;});});
  const initial=context(),view=renderHook(props=>useExactReferenceMeasurement(props),{initialProps:initial,wrapper:strict});
  let pending!:Promise<void>;act(()=>{pending=view.result.current.request();});
  expect(view.result.current.working).toBe(true);
  const project={...initial.project!},next={...initial,project};
  if(changed==="project")project.id="B";
  if(changed==="head"){
    const nextHead="c7861b62-ae24-4d45-a9e5-e29428e05a4f",revision=project.revisions[0],document=JSON.parse(revision.program!);
    document.revisionId=nextHead;project.currentRevision=nextHead;
    project.revisions=[...project.revisions,{...revision,id:nextHead,parent:head,program:JSON.stringify(document)}];
  }
  if(changed==="selection")next.selectionToken="selected-another-edge";
  if(changed==="source")project.files=[{...project.files[0],sha256:"b".repeat(64)}];
  view.rerender(next);expect(view.result.current.report).toBeNull();
  if(changed==="head")expect(view.result.current.available).toBe(true);
  await act(async()=>{finish(report(captured));await pending;});
  expect(view.result.current.report).toBeNull();expect(view.result.current.working).toBe(false);
});
it("unmount closes response adoption while retaining the explicit distinction from native cancellation",async()=>{
  const close=vi.spyOn(MeasurementSession.prototype,"close");
  let finish!:(report:MeasurementReport)=>void,captured!:MeasurementCapture;
  mocked.measure.mockReset().mockImplementation(request=>{captured=request;return new Promise(resolve=>{finish=resolve;});});
  const view=renderHook(props=>useExactReferenceMeasurement(props),{initialProps:context(),wrapper:strict});
  let pending!:Promise<void>;act(()=>{pending=view.result.current.request();});
  const before=view.result.current;view.unmount();await act(async()=>{await Promise.resolve();});
  expect(close).toHaveBeenCalledTimes(1);
  const closedSession=close.mock.contexts[0] as MeasurementSession;
  await act(async()=>{finish(report(captured));await pending;});
  expect(before.report).toBeNull();expect(mocked.measure).toHaveBeenCalledTimes(1);
  expect(closedSession.snapshot().report).toBeNull();close.mockRestore();
});
