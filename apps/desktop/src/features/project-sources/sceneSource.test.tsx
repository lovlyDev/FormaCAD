import {act,render,waitFor} from "@testing-library/react";
import {beforeEach,expect,it,vi} from "vitest";
import * as THREE from "three";
import {CommittedScene} from "../model-apply/committedScene";
import {committedSelectionScene} from "../model-apply/committedSelectionScene";
import {useCommittedScene} from "../model-apply/useCommittedScene";
import {folderSource,refreshFolderSource,type ProjectSourceBinding} from "./sourceBinding";
import {newProject} from "../../stores/workspace";
import {defaults,type Project} from "../../types";
const mock=vi.hoisted(()=>({load:vi.fn()}));
vi.mock("../../lib/files",()=>({loadModel:mock.load}));
const ref=(id="first",commitId="commit")=>({sourceId:id,commitId,taskScopeId:`group-${id}`});
const identity={projectId:"shared",revisionId:"head",sourceSeal:"seal"};
const fixture=()=>{const project={...newProject("Scene","blank","mm","codex"),id:"shared",currentRevision:"head"};
  const revision={id:"head",parent:null,createdAt:project.createdAt,prompt:"fixture",parameters:defaults,program:"{}",source:"source.step",preview:"mesh.glb"};
  return {...project,revisions:[revision],files:[{name:"mesh.glb",kind:"model" as const,size:100,sha256:"a".repeat(64)}]};};
function Host({project,binding}:{project:Project;binding:ProjectSourceBinding}){
  const scene=useCommittedScene(project,project.revisions[0],binding);
  return <div data-testid="scene">{scene.object?.uuid??"empty"}:{String(scene.interactive)}</div>;
}
beforeEach(()=>mock.load.mockReset());
it("does not retain geometry when only the source opening changes",async()=>{
  const dispose=vi.fn(),scene=new CommittedScene<string>(dispose);
  const first={...identity,sourceOpeningId:"first"},second={...identity,sourceOpeningId:"second"};
  await scene.load(first,async()=>"first mesh");let finish!:(v:string)=>void;
  const pending=scene.load(second,()=>new Promise(resolve=>{finish=resolve;}));
  expect(scene.snapshot().object).toBeNull();expect(scene.interactive()).toBe(false);
  expect(committedSelectionScene({phase:"ready",displayed:first,target:first,object:{uuid:"old",userData:{formaNativeCadPreview:true}}},second).interactive).toBe(false);
  finish("second mesh");await pending;scene.releaseRetired();expect(dispose).toHaveBeenCalledWith("first mesh");scene.close();
});
it("disposes a late result from an equal-ID previous source opening",async()=>{
  const dispose=vi.fn(),scene=new CommittedScene<string>(dispose);let finish!:(v:string)=>void;
  const old=scene.load({...identity,sourceOpeningId:"first"},()=>new Promise(resolve=>{finish=resolve;}));
  await scene.load({...identity,sourceOpeningId:"second"},async()=>"current");finish("late");await old;
  expect(scene.snapshot().object).toBe("current");expect(dispose).toHaveBeenCalledWith("late");scene.close();
});
it("does not dispose a cached alias that is adopted by the replacement opening",async()=>{
  const dispose=vi.fn(),scene=new CommittedScene<object>(dispose),object={};
  await scene.load({...identity,sourceOpeningId:"first"},async()=>object);
  await scene.load({...identity,sourceOpeningId:"second"},async()=>object);scene.releaseRetired();expect(dispose).not.toHaveBeenCalled();
  scene.close();expect(dispose).toHaveBeenCalledTimes(1);
});
it("passes the folder reference and hides same-ID old geometry during replacement",async()=>{
  const project=fixture(),first=folderSource(project.id,ref()),second=folderSource(project.id,ref("second"));
  const old=new THREE.Group(),next=new THREE.Group();let finish!:(g:THREE.Group)=>void;
  mock.load.mockResolvedValueOnce(old).mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));
  const view=render(<Host project={project} binding={first}/>);await waitFor(()=>expect(view.getByTestId("scene")).toHaveTextContent(old.uuid));
  view.rerender(<Host project={project} binding={second}/>);expect(view.getByTestId("scene")).toHaveTextContent("empty:false");
  expect(mock.load.mock.calls[1][2]).toEqual(ref("second"));await act(async()=>{finish(next);await Promise.resolve();});
  await waitFor(()=>expect(view.getByTestId("scene")).toHaveTextContent(next.uuid));view.unmount();
});
it("retains ready geometry without loading again on metadata-only commit refresh",async()=>{
  const project=fixture(),first=folderSource(project.id,ref()),object=new THREE.Group();mock.load.mockResolvedValue(object);
  const view=render(<Host project={project} binding={first}/>);await waitFor(()=>expect(view.getByTestId("scene")).toHaveTextContent(object.uuid));
  view.rerender(<Host project={{...project,name:"metadata"}} binding={refreshFolderSource(first,ref("first","new-commit"))}/>);
  await act(async()=>{await Promise.resolve();});expect(mock.load).toHaveBeenCalledTimes(1);expect(view.getByTestId("scene")).toHaveTextContent(object.uuid);view.unmount();
});
it("restarts pending asset IO with the new commit without accepting the older object",async()=>{
  const project=fixture(),first=folderSource(project.id,ref());let finish!:(g:THREE.Group)=>void;
  const fresh=new THREE.Group(),late=new THREE.Group();mock.load.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;})).mockResolvedValueOnce(fresh);
  const view=render(<Host project={project} binding={first}/>);await waitFor(()=>expect(mock.load).toHaveBeenCalledTimes(1));
  view.rerender(<Host project={{...project,name:"metadata"}} binding={refreshFolderSource(first,ref("first","new-commit"))}/>);
  await waitFor(()=>expect(view.getByTestId("scene")).toHaveTextContent(fresh.uuid));await act(async()=>{finish(late);await Promise.resolve();});
  expect(mock.load.mock.calls[1][2]).toEqual(ref("first","new-commit"));expect(view.getByTestId("scene")).toHaveTextContent(fresh.uuid);view.unmount();
});
