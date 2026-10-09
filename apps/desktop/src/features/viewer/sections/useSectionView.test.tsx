import { StrictMode,type ReactNode } from "react";
import { act,cleanup,renderHook } from "@testing-library/react";
import { afterEach,expect,it,vi } from "vitest";
import { Group } from "three";
import { newProject,useWorkspace } from "../../../stores/workspace";
import { defaults } from "../../../types";
import { useSectionView } from "./useSectionView";
import type { SectionReport } from "./sectionApi";
import type { FaceSectionCapture } from "./face-section/faceSectionCapture";
import type { FaceSectionReport } from "./face-section/faceSectionSchema";
const mocked=vi.hoisted(()=>({manual:vi.fn(),face:vi.fn()}));
vi.mock("./sectionApi",()=>({sectionModel:mocked.manual}));
vi.mock("./face-section/faceSectionApi",()=>({sectionReference:mocked.face}));
vi.mock("../../../lib/api",async original=>({...await original<typeof import("../../../lib/api")>(),native:true}));
afterEach(()=>{cleanup();vi.useRealTimers();localStorage.clear();mocked.manual.mockReset();mocked.face.mockReset();useWorkspace.setState({project:null,selectedFace:null});});
const head="c661482b-378a-4fa3-a426-ad9bf6621478",sha="a".repeat(64),scope="forma.ui.project.A.view.main";
function setup(){
 const project=newProject("Section","blank","mm","codex");project.currentRevision=head;
 const program=JSON.stringify({schemaVersion:2,revisionId:head,parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"rectangle"}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile"}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]});
 project.revisions=[{id:head,parent:null,createdAt:project.createdAt,prompt:"Source",parameters:defaults,program,source:"source.step"}];project.files=[{name:"source.step",kind:"model",sha256:sha,size:100}];
 useWorkspace.setState({project,selectedFace:{bodyId:"body",revisionId:head,faceOrdinal:99,topologyRef:{schemaVersion:1,kind:"face",ownerFeatureId:"pad",role:"box-face:zmax",occurrencePath:[]}}});
 localStorage.setItem(`${scope}.section.enabled`,"true");return new Group();
}
function faceReport(capture:FaceSectionCapture):FaceSectionReport{
 return{projectId:capture.request.projectId,schemaVersion:1,requestId:"0fdca3bb-60ac-4014-99e9-28b8306687c7",engine:{name:"occt",protocolVersion:1},evaluationSource:"rebuiltAuthoredFaceAndSealedStep",revisionId:head,bodyId:"body",sourceSha256:sha,sourceSize:100,documentSha256:sha,importedAssetSeals:[],query:capture.request.query,face:{areaMm2:800,originMm:[0,0,10],normal:[0,0,1]},geometry:{plane:{originMm:[0,0,10],normal:[0,0,1],deflectionMm:.01},totalLengthMm:0,curves:[]}};
}
const wrapper=({children}:{children:ReactNode})=><StrictMode>{children}</StrictMode>;
it("switches manual and face modes without enqueuing a second unfinished native invocation",async()=>{
 vi.useFakeTimers();const object=setup();let finishManual:(value:SectionReport)=>void=()=>{},finishFace:(value:FaceSectionReport)=>void=()=>{};
 mocked.manual.mockImplementation(()=>new Promise<SectionReport>(resolve=>{finishManual=resolve;}));mocked.face.mockImplementation(()=>new Promise<FaceSectionReport>(resolve=>{finishFace=resolve;}));
 const view=renderHook(()=>useSectionView(object,scope,false),{wrapper});await act(async()=>{vi.advanceTimersByTime(400);});expect(mocked.manual).toHaveBeenCalledTimes(1);
 await act(async()=>{view.result.current.setMode("selectedFace");vi.advanceTimersByTime(1000);});expect(mocked.face).not.toHaveBeenCalled();expect(view.result.current.clipEnabled).toBe(false);expect(view.result.current.faceSection.available).toBe(true);
 const manual:SectionReport={sourceSha256:sha,geometry:{plane:{originMm:[0,0,0],normal:[0,0,1],deflectionMm:.01},totalLengthMm:0,curves:[]}};
 await act(async()=>{finishManual(manual);});await act(async()=>{vi.advanceTimersByTime(400);});expect(mocked.face).toHaveBeenCalledTimes(1);
 const capture=mocked.face.mock.calls[0][0] as FaceSectionCapture;
 await act(async()=>{view.result.current.setMode("manual");vi.advanceTimersByTime(1000);});expect(mocked.manual).toHaveBeenCalledTimes(1);
 await act(async()=>{finishFace(faceReport(capture));});expect(view.result.current.faceSection.report).toBeUndefined();await act(async()=>{vi.advanceTimersByTime(400);});expect(mocked.manual).toHaveBeenCalledTimes(2);
 await act(async()=>{finishManual(manual);});expect(view.result.current.report).toBe(manual);
});
