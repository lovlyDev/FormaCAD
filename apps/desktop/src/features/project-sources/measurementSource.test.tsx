import {afterEach,expect,it,vi} from "vitest";
import {act,cleanup,renderHook} from "@testing-library/react";
import {newProject} from "../../stores/workspace";
import {defaults} from "../../types";
import {folderSource,refreshFolderSource} from "./sourceBinding";
import {captureMeasurement,type MeasurementCapture,type MeasurementContext} from "../reference-measurements/measurementCapture";
import {MeasurementSession} from "../reference-measurements/measurementSession";
import type {MeasurementReport} from "../reference-measurements/measurementSchema";
import {measureReference} from "../reference-measurements/measurementApi";
import {useExactReferenceMeasurement} from "../reference-measurements/useExactReferenceMeasurement";
const mocks=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/api/core",()=>({invoke:mocks.invoke}));
vi.mock("../../lib/api",()=>({native:true}));
afterEach(()=>{cleanup();vi.resetAllMocks();});
const head="c661482b-378a-4fa3-a426-ad9bf6621478",sha="a".repeat(64);
function fixture():MeasurementContext{
 const project=newProject("Measurement","blank","mm","codex");project.currentRevision=head;
 const program=JSON.stringify({schemaVersion:2,revisionId:head,parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle",width:{kind:"literal",mm:40},depth:{kind:"literal",mm:20}}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile",distance:{kind:"literal",mm:10}}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]});
 project.revisions=[{id:head,parent:null,createdAt:project.createdAt,prompt:"Source",parameters:defaults,program,source:"source.step"}];
 project.files=[{name:"source.step",kind:"model",sha256:sha,size:100}];
 return{project,bodyId:"body",query:{kind:"bodyMetrics"},selectionToken:"body",interactive:true};
}
function source(context:MeasurementContext,id="first"){
 return folderSource(context.project!.id,{sourceId:id,commitId:"commit",taskScopeId:`task-${id}`});
}
function report(capture:MeasurementCapture):MeasurementReport{
 return{projectId:capture.request.projectId,schemaVersion:1,requestId:"0fdca3bb-60ac-4014-99e9-28b8306687c7",engine:{name:"occt",protocolVersion:1},evaluationSource:"rebuiltAuthoredBody",revisionId:head,bodyId:"body",sourceSha256:sha,sourceSize:100,documentSha256:sha,importedAssetSeals:[],query:capture.request.query,result:{kind:"bodyMetrics",volumeMm3:8000,areaMm2:2800,faceCount:6,edgeCount:12,extentsMm:[40,20,10]}};
}
async function nativeReport(capture:MeasurementCapture){
 const hash=await crypto.subtle.digest("SHA-256",new TextEncoder().encode(capture.rawProgram));
 return{...report(capture),documentSha256:Array.from(new Uint8Array(hash),value=>value.toString(16).padStart(2,"0")).join("")};
}
it("distinguishes equal-ID sources/reopens while preserving metadata-compatible geometry identity",()=>{
 const base=fixture(),first=source(base);if(first.kind!=="folder")throw Error("fixture");
 const captured=captureMeasurement({...base,sourceBinding:first})!;
 expect(captureMeasurement({...base,sourceBinding:source(base,"second")})!.key).not.toBe(captured.key);
 expect(captureMeasurement({...base,sourceBinding:source(base)})!.key).not.toBe(captured.key);
 const updated=captureMeasurement({...base,project:{...base.project!,name:"Metadata"},sourceBinding:refreshFolderSource(first,{...first.source,commitId:"metadata"})})!;
 expect(updated.key).toBe(captured.key);expect(updated.source?.commitId).toBe("metadata");expect(captured.source?.commitId).toBe("commit");
});
it("refuses missing/mismatched explicit bindings without managed fallback",()=>{
 const base=fixture();expect(captureMeasurement({...base,sourceBinding:null})).toBeNull();
 expect(captureMeasurement({...base,sourceBinding:folderSource("other",{sourceId:"source",commitId:"commit",taskScopeId:"task"})})).toBeNull();
 expect(captureMeasurement(base)).not.toBeNull();
});
it("does not adopt an old measurement after equal-ID reopen",async()=>{
 const base=fixture(),first=captureMeasurement({...base,sourceBinding:source(base)})!;
 const session=new MeasurementSession();session.setContext(first);let resolve!:(value:MeasurementReport)=>void;
 const pending=session.run(()=>new Promise(done=>{resolve=done;}));
 const reopened=captureMeasurement({...base,sourceBinding:source(base)})!;session.setContext(reopened);resolve(report(first));await pending;
 expect(session.snapshot().report).toBeNull();expect(session.snapshot().key).toBe(reopened.key);
});
it("preserves a compatible pending result while advancing only the next request reference",async()=>{
 const base=fixture(),binding=source(base);if(binding.kind!=="folder")throw Error("fixture");
 const first=captureMeasurement({...base,sourceBinding:binding})!;
 const next=captureMeasurement({...base,sourceBinding:refreshFolderSource(binding,{...binding.source,commitId:"metadata"})})!;
 const session=new MeasurementSession();session.setContext(first);let resolve!:(value:MeasurementReport)=>void;
 const pending=session.run(()=>new Promise(done=>{resolve=done;}));session.setContext(next);resolve(report(first));await pending;
 expect(session.snapshot().report).toEqual(report(first));
 const load=vi.fn(async(capture:MeasurementCapture)=>report(capture));await session.run(load);
 expect(load.mock.calls[0][0]).toBe(next);expect(first.source?.commitId).toBe("commit");
});
it("uses captured folder authority in IPC and refuses an explicit foreign override",async()=>{
 const base=fixture(),captured=captureMeasurement({...base,sourceBinding:source(base)})!;
 mocks.invoke.mockResolvedValue(await nativeReport(captured));await measureReference(captured);
 expect(mocks.invoke).toHaveBeenCalledWith("measure_model_reference",{...captured.request,sourceId:"first",expectedCommitId:"commit"});
 mocks.invoke.mockClear();await expect(measureReference(captured,{sourceId:"other",commitId:"commit",taskScopeId:"task-other"})).rejects.toThrow("MEASUREMENT_STALE");
 expect(mocks.invoke).not.toHaveBeenCalled();
});
it("refreshes hook IPC commit after metadata without hiding a compatible report",async()=>{
 const base=fixture(),binding=source(base);if(binding.kind!=="folder")throw Error("fixture");
 const captured=captureMeasurement({...base,sourceBinding:binding})!;mocks.invoke.mockResolvedValue(await nativeReport(captured));
 const hook=renderHook(({binding})=>useExactReferenceMeasurement({...base,sourceBinding:binding}),{initialProps:{binding}});
 await act(async()=>{await hook.result.current.request();});expect(hook.result.current.report).not.toBeNull();
 hook.rerender({binding:refreshFolderSource(binding,{...binding.source,commitId:"metadata"}) as typeof binding});
 expect(hook.result.current.report).not.toBeNull();await act(async()=>{await hook.result.current.request();});
 expect(mocks.invoke.mock.calls[1][1].expectedCommitId).toBe("metadata");
});
