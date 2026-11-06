import {afterEach,expect,it,vi} from "vitest";
import {act,cleanup,renderHook} from "@testing-library/react";
import {folderSource,refreshFolderSource,type ProjectSourceBinding} from "./sourceBinding";
import {pairTestContext,pairTestReport} from "../pair-measurements/pairTestContext";
import {capturePairScope,capturePair,type PairCapture} from "../pair-measurements/pairCapture";
import {usePairSlots} from "../pair-measurements/usePairSlots";
import {PairSession} from "../pair-measurements/pairSession";
import {measurePair} from "../pair-measurements/pairApi";
import type {PairReport} from "../pair-measurements/pairSchema";
const mocks=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/api/core",()=>({invoke:mocks.invoke}));
vi.mock("../../lib/api",()=>({native:true}));
afterEach(()=>{cleanup();vi.resetAllMocks();});
function fixture(){
 const context=pairTestContext();
 const binding=(id="first")=>folderSource(context.project.id,{sourceId:id,commitId:"commit",taskScopeId:`task-${id}`});
 const capture=(source:ProjectSourceBinding)=>capturePair(capturePairScope(context.project,context.bodyId,context.sceneToken,true,source),context.face.topologyRef!,context.second.topologyRef!,"minimumDistance")!;
 return{context,binding,capture};
}
it("binds pair identity to original source/opening while advancing compatible commit references",()=>{
 const{binding,capture}=fixture(),first=binding();if(first.kind!=="folder")throw Error("fixture");
 const original=capture(first),next=capture(refreshFolderSource(first,{...first.source,commitId:"metadata"}));
 expect(capture(binding("second")).key).not.toBe(original.key);expect(capture(binding()).key).not.toBe(original.key);
 expect(next.key).toBe(original.key);expect(next.source?.commitId).toBe("metadata");expect(original.source?.commitId).toBe("commit");
});
it("retains slot choices through metadata but rejects setters and clear from retired openings",()=>{
 const{context,binding}=fixture(),first=binding();if(first.kind!=="folder")throw Error("fixture");
 const hook=renderHook(({source})=>usePairSlots(context.project,context.bodyId,context.sceneToken,true,null,context.face,source),{initialProps:{source:first as ProjectSourceBinding}});
 act(()=>hook.result.current.captureFirst());expect(hook.result.current.first).toEqual(context.face.topologyRef);
 hook.rerender({source:refreshFolderSource(first,{...first.source,commitId:"metadata"})});expect(hook.result.current.first).toEqual(context.face.topologyRef);
 const staleCapture=hook.result.current.captureSecond,staleClear=hook.result.current.clear;
 hook.rerender({source:binding("second")});expect(hook.result.current.first).toBeNull();
 act(()=>hook.result.current.captureFirst());act(()=>{staleCapture();staleClear();});
 expect(hook.result.current.first).toEqual(context.face.topologyRef);expect(hook.result.current.second).toBeNull();
});
it("never substitutes managed scope for missing or mismatched explicit source",()=>{
 const{context}=fixture();
 expect(capturePairScope(context.project,context.bodyId,context.sceneToken,true,null)).toBeNull();
 const wrong=folderSource("other",{sourceId:"source",commitId:"commit",taskScopeId:"task"});
 expect(capturePairScope(context.project,context.bodyId,context.sceneToken,true,wrong)).toBeNull();
 expect(capturePairScope(context.project,context.bodyId,context.sceneToken,true)).not.toBeNull();
});
it("rejects a late pair result after equal-ID reopen",async()=>{
 const{binding,capture}=fixture(),first=capture(binding()),next=capture(binding());
 const session=new PairSession();session.setContext(first);let resolve!:(value:PairReport)=>void;
 const pending=session.run(()=>new Promise(done=>{resolve=done;}));session.setContext(next);resolve(pairTestReport(first));await pending;
 expect(session.snapshot().report).toBeNull();expect(session.snapshot().key).toBe(next.key);
});
it("preserves compatible pending results and refreshes only the next request capture",async()=>{
 const{binding,capture}=fixture(),source=binding();if(source.kind!=="folder")throw Error("fixture");
 const first=capture(source),next=capture(refreshFolderSource(source,{...source.source,commitId:"metadata"}));
 const session=new PairSession();session.setContext(first);let resolve!:(value:PairReport)=>void;
 const pending=session.run(()=>new Promise(done=>{resolve=done;}));session.setContext(next);resolve(pairTestReport(first));await pending;
 expect(session.snapshot().report).toEqual(pairTestReport(first));
 const load=vi.fn(async(value:PairCapture)=>pairTestReport(value));await session.run(load);expect(load.mock.calls[0][0]).toBe(next);
});
it("routes native pair IPC through its captured folder and refuses foreign overrides",async()=>{
 const{binding,capture}=fixture(),captured=capture(binding());
 const hash=await crypto.subtle.digest("SHA-256",new TextEncoder().encode(captured.rawProgram));
 mocks.invoke.mockResolvedValue({...pairTestReport(captured),documentSha256:Array.from(new Uint8Array(hash),value=>value.toString(16).padStart(2,"0")).join("")});
 await measurePair(captured);expect(mocks.invoke).toHaveBeenCalledWith("measure_model_pair",{...captured.request,sourceId:"first",expectedCommitId:"commit"});
 mocks.invoke.mockClear();await expect(measurePair(captured,{sourceId:"other",commitId:"commit",taskScopeId:"other-task"})).rejects.toThrow("MEASUREMENT_STALE");
 expect(mocks.invoke).not.toHaveBeenCalled();
});
