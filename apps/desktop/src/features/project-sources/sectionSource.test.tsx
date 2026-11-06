import {act,cleanup,renderHook} from "@testing-library/react";
import {afterEach,expect,it,vi} from "vitest";
import {Group} from "three";
import {useWorkspace} from "../../stores/workspace";
import {pairTestContext} from "../pair-measurements/pairTestContext";
import {captureSceneRead,rememberSceneRead} from "./sceneRead";
import {captureSectionSource} from "../viewer/sections/sectionSource";
import {useSectionView} from "../viewer/sections/useSectionView";
import type {SectionReport} from "../viewer/sections/sectionApi";
const mocks=vi.hoisted(()=>({manual:vi.fn(),face:vi.fn(),invoke:vi.fn()}));
vi.mock("@tauri-apps/api/core",async original=>({...await original<typeof import("@tauri-apps/api/core")>(),invoke:mocks.invoke}));
vi.mock("../viewer/sections/sectionApi",()=>({sectionModel:mocks.manual}));
vi.mock("../viewer/sections/face-section/faceSectionApi",()=>({sectionReference:mocks.face}));
vi.mock("../../lib/api",async original=>({...await original<typeof import("../../lib/api")>(),native:true}));
afterEach(()=>{cleanup();vi.useRealTimers();vi.resetAllMocks();localStorage.clear();useWorkspace.getState().setProject(null);});
const scope="forma.ui.section-source";
function fixture(){
 const context=pairTestContext();
 const open=(id="first")=>{
  useWorkspace.getState().openFolderProject(context.project,{sourceId:id,commitId:"commit",taskScopeId:`task-${id}`});
  const object=new Group(),state=useWorkspace.getState();rememberSceneRead(object,captureSceneRead(state.project,state.sourceBinding));return object;
 };
 return{context,open,object:open()};
}
const report=():SectionReport=>({sourceSha256:"a".repeat(64),geometry:{plane:{originMm:[0,0,0],normal:[0,0,1],deflectionMm:.01},totalLengthMm:120,curves:[]}});
async function start(view:ReturnType<typeof renderHook<ReturnType<typeof useSectionView>,{object:Group}>>){
 await act(async()=>view.result.current.setEnabled(true));await act(async()=>vi.advanceTimersByTime(400));
}
it("routes production manual sections through the displayed folder original commit",async()=>{
 vi.useFakeTimers();const f=fixture();mocks.manual.mockResolvedValue(report());
 const view=renderHook(({object})=>useSectionView(object,scope,false),{initialProps:{object:f.object}});await start(view);
 expect(mocks.manual).toHaveBeenCalledTimes(1);expect(mocks.manual.mock.calls[0][4]).toEqual({sourceId:"first",commitId:"commit",taskScopeId:"task-first"});
 expect(view.result.current.report).toEqual(report());
});
it.each(["resolve","reject"] as const)("rejects late manual %s after equal-ID source replacement",async outcome=>{
 vi.useFakeTimers();const f=fixture();let finish!:(value:SectionReport)=>void,fail!:(error:Error)=>void;
 mocks.manual.mockImplementationOnce(()=>new Promise<SectionReport>((resolve,reject)=>{finish=resolve;fail=reject;})).mockResolvedValue(report());
 const view=renderHook(({object})=>useSectionView(object,scope,false),{initialProps:{object:f.object}});await start(view);
 act(()=>view.rerender({object:f.open("second")}));
 await act(async()=>{if(outcome==="resolve")finish(report());else fail(Error("old source"));});
 expect(view.result.current.report).toBeUndefined();expect(view.result.current.error).toBeNull();expect(view.result.current.enabled).toBe(false);
 await start(view);expect(mocks.manual.mock.calls[1][4].sourceId).toBe("second");expect(view.result.current.report).toEqual(report());
});
it("restarts strict manual metadata requests with the next commit without retargeting the running request",async()=>{
 vi.useFakeTimers();const f=fixture();let finish!:(value:SectionReport)=>void;
 mocks.manual.mockImplementationOnce(()=>new Promise<SectionReport>(resolve=>{finish=resolve;})).mockResolvedValue(report());
 const view=renderHook(({object})=>useSectionView(object,scope,false),{initialProps:{object:f.object}});await start(view);
 const state=useWorkspace.getState(),binding=state.sourceBinding!;if(binding.kind!=="folder")throw Error("fixture");
 act(()=>{expect(useWorkspace.getState().publishFolderSnapshot({...f.context.project,name:"Metadata"},binding,{...binding.source,commitId:"metadata"})).toBe(true);});
 expect(mocks.manual.mock.calls[0][4].commitId).toBe("commit");await act(async()=>finish(report()));
 expect(view.result.current.report).toBeUndefined();await act(async()=>vi.advanceTimersByTime(400));
 expect(mocks.manual.mock.calls[1][4].commitId).toBe("metadata");expect(view.result.current.report).toEqual(report());
});
it("does not borrow folder routing for an unregistered or retired displayed object",()=>{
 const f=fixture();let state=useWorkspace.getState();expect(captureSectionSource(new Group(),state.project,state.sourceBinding)).toBeNull();
 const original=captureSectionSource(f.object,state.project,state.sourceBinding)!;expect(original.isCurrent()).toBe(true);
 f.open("second");state=useWorkspace.getState();expect(captureSectionSource(f.object,state.project,state.sourceBinding)?.isOpeningCurrent()).toBe(false);
 expect(original.isOpeningCurrent()).toBe(false);expect(original.isSnapshotCurrent()).toBe(false);
});
it("retains only local clipping during same-opening geometry replacement and suspends new native work",async()=>{
 vi.useFakeTimers();const f=fixture();mocks.manual.mockResolvedValue(report());
 const view=renderHook(()=>useSectionView(f.object,scope,false,true));await act(async()=>view.result.current.setEnabled(true));
 const state=useWorkspace.getState(),binding=state.sourceBinding!;if(binding.kind!=="folder")throw Error("fixture");
 act(()=>{useWorkspace.getState().publishFolderSnapshot({...f.context.project,files:f.context.project.files.map(file=>({...file,sha256:"b".repeat(64)}))},binding,{...binding.source,commitId:"geometry"});});
 expect(view.result.current.clipEnabled).toBe(true);expect(view.result.current.available).toBe(false);
 await act(async()=>vi.advanceTimersByTime(1000));expect(mocks.manual).not.toHaveBeenCalled();
});
it("keeps folder section preferences in RAM and refuses stale setters after reopening",async()=>{
 vi.useFakeTimers();const f=fixture();localStorage.setItem(`${scope}.section.enabled`,"true");
 const view=renderHook(({object})=>useSectionView(object,scope,false),{initialProps:{object:f.object}});
 expect(view.result.current.enabled).toBe(false);const stale=view.result.current.setOffset;
 await act(async()=>{view.result.current.setOffset(9);view.result.current.setFaceOffset(3);});expect(view.result.current.offset).toBe(9);
 act(()=>view.rerender({object:f.open("second")}));act(()=>stale(77));
 expect(view.result.current.offset).toBe(0);expect(view.result.current.faceOffset).toBe(0);expect(localStorage.getItem(`${scope}.section.offsetMm`)).toBeNull();
});
it("passes the folder opening into the production selected-face section caller",async()=>{
 vi.useFakeTimers();const f=fixture();useWorkspace.getState().setSelectedFace(f.context.face);
 mocks.face.mockImplementation(async capture=>({projectId:capture.request.projectId,revisionId:capture.request.expectedRevision,geometry:{plane:{originMm:[0,0,0],normal:[0,0,1],deflectionMm:.01},totalLengthMm:120,curves:[]}}));
 const view=renderHook(()=>useSectionView(f.object,scope,false));
 await act(async()=>{view.result.current.setEnabled(true);view.result.current.setMode("selectedFace");});await act(async()=>vi.advanceTimersByTime(400));
 expect(mocks.face).toHaveBeenCalledTimes(1);expect(mocks.face.mock.calls[0][0].source).toEqual({sourceId:"first",commitId:"commit",taskScopeId:"task-first"});
 expect(view.result.current.faceSection.report).toBeDefined();
});
it("routes the manual native API to its original source and rejects mismatched STEP reports",async()=>{
 const api=await vi.importActual<typeof import("../viewer/sections/sectionApi")>("../viewer/sections/sectionApi");
 const source={sourceId:"first",commitId:"commit",taskScopeId:"task-first"},value=report();mocks.invoke.mockResolvedValue(value);
 await api.sectionModel("project","revision",value.sourceSha256,value.geometry.plane,source);
 expect(mocks.invoke).toHaveBeenCalledWith("section_model",{projectId:"project",expectedRevision:"revision",...value.geometry.plane,sourceId:"first",expectedCommitId:"commit"});
 mocks.invoke.mockResolvedValue({...value,sourceSha256:"b".repeat(64)});
 await expect(api.sectionModel("project","revision",value.sourceSha256,value.geometry.plane,source)).rejects.toThrow("SECTION_STALE");
});
