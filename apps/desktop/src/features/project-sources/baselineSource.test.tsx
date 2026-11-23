import { afterEach, expect, it, vi } from "vitest";
import { act, cleanup, renderHook } from "@testing-library/react";
import { Group } from "three";
import { newProject } from "../../stores/workspace";
import { defaults } from "../../types";
import { folderSource, type ProjectSourceBinding } from "./sourceBinding";
import { useSavedBaseline } from "../agents/review/useSavedBaseline";
const mocks=vi.hoisted(()=>({load:vi.fn(),dispose:vi.fn()}));
vi.mock("../../lib/files",()=>({loadModel:mocks.load}));
vi.mock("../../lib/model",()=>({disposeModel:mocks.dispose}));
afterEach(()=>{cleanup();vi.resetAllMocks();});
function fixture(){
 const project=newProject("Review","blank","mm","codex");project.currentRevision="base";
 project.revisions=[{id:"base",parent:null,createdAt:project.createdAt,prompt:"",parameters:defaults,preview:"base.glb"}];
 project.files=[{name:"base.glb",kind:"model",size:100,sha256:"a".repeat(64)}];
 const binding=()=>folderSource(project.id,{sourceId:"source",commitId:"commit",taskScopeId:"task"});
 return{project,binding};
}
it("loads the baseline from its captured host folder reference",async()=>{
 const{project,binding}=fixture(),source=binding(),object=new Group();mocks.load.mockResolvedValue(object);
 const hook=renderHook(()=>useSavedBaseline(project,source));await act(async()=>{});
 expect(mocks.load).toHaveBeenCalledWith(project.files[0],project.id,source.kind==="folder"?source.source:null);
 expect(hook.result.current.object).toBe(object);
});
it("hides old baseline and disposes late model after equal-ID reopen",async()=>{
 const{project,binding}=fixture();let resolve!:(object:Group)=>void;mocks.load.mockImplementationOnce(()=>new Promise<Group>(done=>{resolve=done;})).mockResolvedValueOnce(new Group());
 const hook=renderHook(({source})=>useSavedBaseline(project,source),{initialProps:{source:binding()}});
 hook.rerender({source:binding()});expect(hook.result.current.object).toBeNull();
 const late=new Group();await act(async()=>resolve(late));expect(mocks.dispose).toHaveBeenCalledWith(late);
 expect(hook.result.current.object).not.toBe(late);
});
it("does not route a missing or mismatched explicit source to managed loading",async()=>{
 const{project}=fixture(),wrong=folderSource("other",{sourceId:"source",commitId:"commit",taskScopeId:"task"});
 const hook=renderHook(({source})=>useSavedBaseline(project,source),{initialProps:{source:wrong as ProjectSourceBinding|null}});
 await act(async()=>{});hook.rerender({source:null});await act(async()=>{});
 expect(mocks.load).not.toHaveBeenCalled();expect(hook.result.current.pending).toBe(false);
});
