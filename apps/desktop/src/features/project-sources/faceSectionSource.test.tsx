import {afterEach,expect,it,vi} from "vitest";
import {act,cleanup,renderHook} from "@testing-library/react";
import {folderSource,refreshFolderSource,type ProjectSourceBinding} from "./sourceBinding";
import {pairTestContext} from "../pair-measurements/pairTestContext";
import {captureFaceSection,type FaceSectionCapture} from "../viewer/sections/face-section/faceSectionCapture";
import {useFaceSection} from "../viewer/sections/face-section/useFaceSection";
import type {FaceSectionReport} from "../viewer/sections/face-section/faceSectionSchema";
import * as api from "../viewer/sections/face-section/faceSectionApi";
const mocks=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/api/core",()=>({invoke:mocks.invoke}));
vi.mock("../../lib/api",()=>({native:true}));
afterEach(()=>{cleanup();vi.restoreAllMocks();vi.resetAllMocks();vi.useRealTimers();});
function fixture(){
 const context=pairTestContext();const binding=(id="first")=>folderSource(context.project.id,{sourceId:id,commitId:"commit",taskScopeId:`task-${id}`});
 const capture=(source:ProjectSourceBinding)=>captureFaceSection(context.project,context.face,-2,true,source)!;
 return{context,binding,capture};
}
function report(capture:FaceSectionCapture):FaceSectionReport{
 return{projectId:capture.request.projectId,schemaVersion:1,requestId:"0fdca3bb-60ac-4014-99e9-28b8306687c7",engine:{name:"occt",protocolVersion:1},evaluationSource:"rebuiltAuthoredFaceAndSealedStep",revisionId:capture.request.expectedRevision,bodyId:capture.request.bodyId,sourceSha256:capture.sourceSha256,sourceSize:capture.sourceSize,documentSha256:"a".repeat(64),importedAssetSeals:[],query:capture.request.query,face:{areaMm2:800,originMm:[0,0,10],normal:[0,0,1]},geometry:{plane:{originMm:[0,0,10+capture.request.query.offsetMm],normal:[0,0,1],deflectionMm:.01},totalLengthMm:0,curves:[]}};
}
it("separates equal-ID openings while preserving compatible face geometry identity",()=>{
 const{binding,capture}=fixture(),source=binding();if(source.kind!=="folder")throw Error("fixture");
 const original=capture(source),next=capture(refreshFolderSource(source,{...source.source,commitId:"metadata"}));
 expect(capture(binding("second")).key).not.toBe(original.key);expect(capture(binding()).key).not.toBe(original.key);
 expect(next.key).toBe(original.key);expect(next.source?.commitId).toBe("metadata");expect(original.source?.commitId).toBe("commit");
});
it("refuses absent or mismatched explicit bindings without managed fallback",()=>{
 const{context}=fixture();expect(captureFaceSection(context.project,context.face,-2,true,null)).toBeNull();
 const wrong=folderSource("other",{sourceId:"source",commitId:"commit",taskScopeId:"task"});
 expect(captureFaceSection(context.project,context.face,-2,true,wrong)).toBeNull();
 expect(captureFaceSection(context.project,context.face,-2,true)).not.toBeNull();
});
it("never clips with a late result after reopening an identical source",async()=>{
 vi.useFakeTimers();const{context,binding}=fixture();let resolve!:(value:FaceSectionReport)=>void;
 const load=vi.spyOn(api,"sectionReference").mockImplementationOnce(()=>new Promise(done=>{resolve=done;}));
 const hook=renderHook(({source})=>useFaceSection(context.project,context.face,-2,true,context.sceneToken,true,false,source),{initialProps:{source:binding()}});
 await act(async()=>vi.advanceTimersByTime(400));const old=load.mock.calls[0][0];hook.rerender({source:binding()});
 await act(async()=>resolve(report(old)));expect(hook.result.current.plane).toBeUndefined();expect(hook.result.current.report).toBeUndefined();
});
it("retains compatible face report through metadata and uses fresh commit on explicit refresh",async()=>{
 vi.useFakeTimers();const{context,binding}=fixture(),source=binding();if(source.kind!=="folder")throw Error("fixture");
 const load=vi.spyOn(api,"sectionReference").mockImplementation(async capture=>report(capture));
 const hook=renderHook(({source})=>useFaceSection(context.project,context.face,-2,true,context.sceneToken,true,false,source),{initialProps:{source:source as ProjectSourceBinding}});
 await act(async()=>vi.advanceTimersByTime(400));const previous=hook.result.current.report;
 hook.rerender({source:refreshFolderSource(source,{...source.source,commitId:"metadata"})});
 expect(hook.result.current.report).toBe(previous);expect(load).toHaveBeenCalledTimes(1);
 act(()=>hook.result.current.calculate());await act(async()=>vi.advanceTimersByTime(0));
 expect(load.mock.calls[1][0].source?.commitId).toBe("metadata");expect(hook.result.current.plane?.originMm).toEqual([0,0,8]);
});
it("ignores a calculate callback belonging to a retired opening",async()=>{
 vi.useFakeTimers();const{context,binding}=fixture();const load=vi.spyOn(api,"sectionReference").mockImplementation(async capture=>report(capture));
 const hook=renderHook(({source})=>useFaceSection(context.project,context.face,-2,true,context.sceneToken,false,false,source),{initialProps:{source:binding()}});
 const stale=hook.result.current.calculate;hook.rerender({source:binding("second")});act(()=>stale());await act(async()=>vi.advanceTimersByTime(0));
 expect(load).not.toHaveBeenCalled();act(()=>hook.result.current.calculate());await act(async()=>vi.advanceTimersByTime(0));
 expect(load).toHaveBeenCalledTimes(1);expect(load.mock.calls[0][0].source?.sourceId).toBe("second");
});
it("uses captured folder routing in native face-section API and refuses foreign overrides",async()=>{
 const{binding,capture}=fixture(),captured=capture(binding());const hash=await crypto.subtle.digest("SHA-256",new TextEncoder().encode(captured.rawProgram));
 mocks.invoke.mockResolvedValue({...report(captured),documentSha256:Array.from(new Uint8Array(hash),byte=>byte.toString(16).padStart(2,"0")).join("")});
 await api.sectionReference(captured);expect(mocks.invoke).toHaveBeenCalledWith("section_model_reference",{...captured.request,sourceId:"first",expectedCommitId:"commit"});
 mocks.invoke.mockClear();await expect(api.sectionReference(captured,{sourceId:"other",commitId:"commit",taskScopeId:"other-task"})).rejects.toThrow("SECTION_STALE");expect(mocks.invoke).not.toHaveBeenCalled();
});
