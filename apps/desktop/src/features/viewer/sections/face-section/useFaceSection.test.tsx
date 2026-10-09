import { StrictMode,type ReactNode } from "react";
import { act,cleanup,renderHook } from "@testing-library/react";
import { afterEach,expect,it,vi } from "vitest";
import { newProject } from "../../../../stores/workspace";
import { defaults } from "../../../../types";
import type { FaceSelection } from "../../faceSelection";
import { useFaceSection } from "./useFaceSection";
import type { FaceSectionCapture } from "./faceSectionCapture";
import type { FaceSectionReport } from "./faceSectionSchema";
const mocked=vi.hoisted(()=>({section:vi.fn()}));
vi.mock("./faceSectionApi",()=>({sectionReference:mocked.section}));
afterEach(()=>{cleanup();vi.useRealTimers();mocked.section.mockReset();});
const head="c661482b-378a-4fa3-a426-ad9bf6621478",sha="a".repeat(64);
function context(){
 const project=newProject("Face section","blank","mm","codex");project.currentRevision=head;
 const program=JSON.stringify({schemaVersion:2,revisionId:head,parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle"}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile"}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]});
 project.revisions=[{id:head,parent:null,createdAt:project.createdAt,prompt:"Source",parameters:defaults,program,source:"source.step"}];project.files=[{name:"source.step",kind:"model",sha256:sha,size:100}];
 const selection:FaceSelection={bodyId:"body",revisionId:head,faceOrdinal:99,topologyRef:{schemaVersion:1,kind:"face",ownerFeatureId:"pad",role:"box-face:zmax",occurrencePath:[]}};
 return{project,selection,offsetMm:-2,active:true,sceneToken:"scene-A"};
}
function report(capture:FaceSectionCapture):FaceSectionReport{
 return{projectId:capture.request.projectId,schemaVersion:1,requestId:"0fdca3bb-60ac-4014-99e9-28b8306687c7",engine:{name:"occt",protocolVersion:1},evaluationSource:"rebuiltAuthoredFaceAndSealedStep",revisionId:head,bodyId:"body",sourceSha256:sha,sourceSize:100,documentSha256:sha,importedAssetSeals:[],query:capture.request.query,face:{areaMm2:800,originMm:[0,0,10],normal:[0,0,1]},geometry:{plane:{originMm:[0,0,10+capture.request.query.offsetMm],normal:[0,0,1],deflectionMm:.01},totalLengthMm:0,curves:[]}};
}
const strict=({children}:{children:ReactNode})=><StrictMode>{children}</StrictMode>;
it("does not adopt a late offset result and starts only the latest queued context in StrictMode",async()=>{
 vi.useFakeTimers();let finish:(value:FaceSectionReport)=>void=()=>{};
 mocked.section.mockImplementationOnce(()=>new Promise<FaceSectionReport>(resolve=>{finish=resolve;})).mockImplementation(async(capture:FaceSectionCapture)=>report(capture));
 const initial=context(),view=renderHook(props=>useFaceSection(props.project,props.selection,props.offsetMm,props.active,props.sceneToken),{initialProps:initial,wrapper:strict});
 await act(async()=>{vi.advanceTimersByTime(400);});expect(mocked.section).toHaveBeenCalledTimes(1);
 const captured=mocked.section.mock.calls[0][0] as FaceSectionCapture;
 view.rerender({...initial,offsetMm:-3});expect(view.result.current.plane).toBeUndefined();expect(view.result.current.working).toBe(true);
 await act(async()=>{finish(report(captured));});expect(view.result.current.report).toBeUndefined();
 await act(async()=>{vi.advanceTimersByTime(400);});expect(mocked.section).toHaveBeenCalledTimes(2);expect(view.result.current.plane?.originMm).toEqual([0,0,7]);
 view.rerender({...initial,offsetMm:-3,sceneToken:"scene-B"});expect(view.result.current.plane).toBeUndefined();
});
it("inactive or unmounted views never adopt an unfinished native section",async()=>{
 vi.useFakeTimers();let finish:(value:FaceSectionReport)=>void=()=>{};mocked.section.mockImplementation(()=>new Promise<FaceSectionReport>(resolve=>{finish=resolve;}));
 const initial=context(),view=renderHook(props=>useFaceSection(props.project,props.selection,props.offsetMm,props.active,props.sceneToken),{initialProps:initial,wrapper:strict});
 await act(async()=>{vi.advanceTimersByTime(400);});const capture=mocked.section.mock.calls[0][0] as FaceSectionCapture;
 view.rerender({...initial,active:false});expect(view.result.current.plane).toBeUndefined();expect(view.result.current.report).toBeUndefined();view.unmount();
 await act(async()=>{finish(report(capture));});expect(mocked.section).toHaveBeenCalledTimes(1);
});
it("explicit calculation starts immediately with automatic updates off and preserves verified same-context clipping while pending",async()=>{
 vi.useFakeTimers();let reject:(error:Error)=>void=()=>{};
 mocked.section.mockImplementationOnce(async(capture:FaceSectionCapture)=>report(capture)).mockImplementationOnce(()=>new Promise<FaceSectionReport>((_resolve,fail)=>{reject=fail;}));
 const initial={...context(),automatic:false},view=renderHook(props=>useFaceSection(props.project,props.selection,props.offsetMm,props.active,props.sceneToken,props.automatic),{initialProps:initial,wrapper:strict});
 await act(async()=>{vi.advanceTimersByTime(1000);});expect(mocked.section).not.toHaveBeenCalled();expect(view.result.current.pending).toBe(false);
 await act(async()=>{view.result.current.calculate();});await act(async()=>{vi.advanceTimersByTime(0);});expect(mocked.section).toHaveBeenCalledTimes(1);expect(view.result.current.plane?.originMm).toEqual([0,0,8]);
 const previous=view.result.current.report;await act(async()=>{view.result.current.calculate();});await act(async()=>{vi.advanceTimersByTime(0);});expect(mocked.section).toHaveBeenCalledTimes(2);expect(view.result.current.report).toBe(previous);expect(view.result.current.working).toBe(true);
 await act(async()=>{view.result.current.calculate();});expect(mocked.section).toHaveBeenCalledTimes(2);
 await act(async()=>{reject(Error("SECTION_STALE"));});expect(view.result.current.plane).toBeUndefined();expect(view.result.current.error).toBeDefined();
 view.rerender({...initial,offsetMm:-3});await act(async()=>{vi.advanceTimersByTime(1000);});expect(view.result.current.report).toBeUndefined();expect(mocked.section).toHaveBeenCalledTimes(2);
});
it.each(["project","head","source","selection"] as const)("a late %s response never clips a new still-valid context",async(change)=>{
 vi.useFakeTimers();let finish:(value:FaceSectionReport)=>void=()=>{};mocked.section.mockImplementation(()=>new Promise<FaceSectionReport>(resolve=>{finish=resolve;}));
 const initial=context(),view=renderHook(props=>useFaceSection(props.project,props.selection,props.offsetMm,props.active,props.sceneToken),{initialProps:initial,wrapper:strict});
 await act(async()=>{vi.advanceTimersByTime(400);});const captured=mocked.section.mock.calls[0][0] as FaceSectionCapture;
 const next=structuredClone(initial);
 if(change==="project")next.project.id="other_project";
 if(change==="source")next.project.files[0].sha256="b".repeat(64);
 if(change==="selection")next.selection.topologyRef={...next.selection.topologyRef!,role:"box-face:zmin"};
 if(change==="head"){
  const newer="5f160704-6109-4a7b-93ef-05e64c87a0c0";next.project.currentRevision=newer;next.project.revisions[0].id=newer;next.selection.revisionId=newer;
  const document=JSON.parse(next.project.revisions[0].program!);document.revisionId=newer;next.project.revisions[0].program=JSON.stringify(document);
 }
 view.rerender(next);expect(view.result.current.available).toBe(true);expect(view.result.current.plane).toBeUndefined();
 await act(async()=>{finish(report(captured));});expect(view.result.current.plane).toBeUndefined();await act(async()=>{vi.advanceTimersByTime(400);});expect(mocked.section).toHaveBeenCalledTimes(2);expect(view.result.current.report).toBeUndefined();
});
it("never transfers an explicit calculate intent to another context before its 0ms timer launches",async()=>{
 vi.useFakeTimers();mocked.section.mockImplementation(async(capture:FaceSectionCapture)=>report(capture));
 const initial={...context(),automatic:false},view=renderHook(props=>useFaceSection(props.project,props.selection,props.offsetMm,props.active,props.sceneToken,props.automatic),{initialProps:initial,wrapper:strict});
 await act(async()=>{view.result.current.calculate();});view.rerender({...initial,project:{...initial.project,id:"new_project"}});
 await act(async()=>{vi.advanceTimersByTime(1000);});expect(view.result.current.available).toBe(true);expect(mocked.section).not.toHaveBeenCalled();expect(view.result.current.plane).toBeUndefined();
});
