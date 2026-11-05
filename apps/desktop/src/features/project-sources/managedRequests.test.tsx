import {act,cleanup,renderHook,waitFor} from "@testing-library/react";
import {QueryClient,QueryClientProvider} from "@tanstack/react-query";
import type {ReactNode} from "react";
import {afterEach,beforeEach,expect,it,vi} from "vitest";
import {captureApply,executeCapturedApply} from "../model-apply/capturedApply";
import {useProjectNavigation} from "../project-catalog/useProjectNavigation";
import {useRevisionEvents} from "../model-apply/useRevisionEvents";
import {useModelApply} from "../model-apply/useModelApply";
import {newProject,useWorkspace} from "../../stores/workspace";
import type {Project} from "../../types";
const mock=vi.hoisted(()=>({read:vi.fn(),apply:vi.fn(),save:vi.fn(),listen:vi.fn(),close:vi.fn()}));
vi.mock("../../lib/api",()=>({native:true,applyProgram:mock.apply,saveProject:mock.save}));
vi.mock("../project-catalog/api",()=>({readProjectSnapshot:mock.read}));
vi.mock("@tauri-apps/api/event",()=>({listen:mock.listen}));
vi.mock("./sourceApi",()=>({closeFolderSource:mock.close}));
const ref={sourceId:"folder",commitId:"commit",taskScopeId:"group"};
const project=()=>({...newProject("Source","blank","mm","codex"),id:"shared",currentRevision:"head"});
function harness(){const query=new QueryClient({defaultOptions:{queries:{retry:false}}});return{query,wrapper:({children}:{children:ReactNode})=><QueryClientProvider client={query}>{children}</QueryClientProvider>};}
beforeEach(()=>{mock.close.mockResolvedValue(true);useWorkspace.getState().setProject(null);useWorkspace.getState().setBusy(false);localStorage.clear();mock.read.mockReset();mock.apply.mockReset();mock.save.mockReset();mock.listen.mockReset();});
afterEach(()=>cleanup());
it("refuses an old approval after equal-ID reopen before invoking CAD",async()=>{
  const base={id:"shared",currentRevision:"head"},request=captureApply(base,"draft","edit","old");const invoke=vi.fn();
  await expect(executeCapturedApply(request,{getCurrent:()=>base,getSourceOpeningId:()=>"new",invoke,adopt:vi.fn(),reconcileTarget:vi.fn(),refreshTarget:vi.fn()})).rejects.toThrow("MODEL_APPLY_BASE_CHANGED");
  expect(invoke).not.toHaveBeenCalled();
});
it("keeps an accepted native result orphaned after source-family change with equal IDs",async()=>{
  const base={id:"shared",currentRevision:"head"},request=captureApply(base,"draft","edit","old");let opening="old";
  const adopt=vi.fn(),reconcile=vi.fn(async()=>{});
  const outcome=await executeCapturedApply(request,{getCurrent:()=>base,getSourceOpeningId:()=>opening,invoke:async()=>{opening="new";return{...base,currentRevision:"accepted"};},adopt,reconcileTarget:reconcile,refreshTarget:vi.fn()});
  expect(outcome.disposition).toBe("orphaned");expect(outcome.result.currentRevision).toBe("accepted");expect(adopt).not.toHaveBeenCalled();expect(reconcile).toHaveBeenCalledTimes(1);
});
it("does not replace a folder view with a late equal-ID managed card read",async()=>{
  const p=project(),navigate=vi.fn(),error=vi.fn();useWorkspace.getState().setProject(p);
  let finish!:(p:Project)=>void;mock.read.mockImplementation(()=>new Promise(resolve=>{finish=resolve;}));
  const view=renderHook(()=>useProjectNavigation(p.id,navigate,error));let pending!:Promise<void>;
  act(()=>{pending=view.result.current(p);});act(()=>useWorkspace.getState().openFolderProject(p,ref));const binding=useWorkspace.getState().sourceBinding;
  await act(async()=>{finish({...p,name:"managed response"});await pending;});
  expect(useWorkspace.getState().project).toBe(p);expect(useWorkspace.getState().sourceBinding).toBe(binding);expect(navigate).not.toHaveBeenCalled();expect(error).not.toHaveBeenCalled();view.unmount();
});
it("does not read a managed snapshot for an ID-only revision event targeting the active folder",async()=>{
  const p=project();useWorkspace.getState().openFolderProject(p,ref);let event!:(e:{payload:string})=>void;
  mock.listen.mockImplementation(async(_name,listener)=>{event=listener;return vi.fn();});const {query,wrapper}=harness();
  const view=renderHook(()=>useRevisionEvents(),{wrapper});await waitFor(()=>expect(event).toBeDefined());act(()=>event({payload:p.id}));
  expect(mock.read).not.toHaveBeenCalled();expect(useWorkspace.getState().project).toBe(p);view.unmount();query.clear();
});
it("does not adopt an in-flight managed revision event after equal-ID folder switch",async()=>{
  const p=project();useWorkspace.getState().setProject(p);let event!:(e:{payload:string})=>void,finish!:(p:Project)=>void;
  mock.listen.mockImplementation(async(_name,listener)=>{event=listener;return vi.fn();});mock.read.mockImplementation(()=>new Promise(resolve=>{finish=resolve;}));
  const {query,wrapper}=harness();const view=renderHook(()=>useRevisionEvents(),{wrapper});await waitFor(()=>expect(event).toBeDefined());act(()=>event({payload:p.id}));
  act(()=>useWorkspace.getState().openFolderProject(p,ref));const binding=useWorkspace.getState().sourceBinding;
  await act(async()=>{finish({...p,currentRevision:"managed-new"});await Promise.resolve();});
  expect(useWorkspace.getState().project).toBe(p);expect(useWorkspace.getState().sourceBinding).toBe(binding);view.unmount();query.clear();
});
it("keeps a completed managed CAD result out of an equal-ID folder and skips its metadata save",async()=>{
  const p=project();useWorkspace.getState().setProject(p);const request=captureApply(p,"draft","edit",useWorkspace.getState().sourceBinding!.openingId);
  let finish!:(p:Project)=>void;mock.apply.mockImplementation(()=>new Promise(resolve=>{finish=resolve;}));mock.read.mockResolvedValue({...p,currentRevision:"accepted"});
  const {query,wrapper}=harness();const view=renderHook(()=>useModelApply(),{wrapper});let pending!:ReturnType<ReturnType<typeof useModelApply>>;
  act(()=>{pending=view.result.current(request,{assistantMessage:"message"});});act(()=>useWorkspace.getState().openFolderProject(p,ref));const binding=useWorkspace.getState().sourceBinding;
  let outcome!:Awaited<typeof pending>;await act(async()=>{finish({...p,currentRevision:"accepted"});outcome=await pending;});
  expect(outcome.disposition).toBe("orphaned");expect(outcome.metadataFailure).toBeDefined();expect(mock.save).not.toHaveBeenCalled();expect(useWorkspace.getState().project).toBe(p);expect(useWorkspace.getState().sourceBinding).toBe(binding);
  view.unmount();query.clear();
});
it("does not invoke the managed CAD route for an active folder reference",async()=>{
  const p=project();useWorkspace.getState().openFolderProject(p,ref);const request=captureApply(p,"draft","edit",useWorkspace.getState().sourceBinding!.openingId);
  const {query,wrapper}=harness();const view=renderHook(()=>useModelApply(),{wrapper});
  await act(async()=>{await expect(view.result.current(request)).rejects.toThrow("PROJECT_SOURCE_WRITE_NOT_AUTHORIZED");});expect(mock.apply).not.toHaveBeenCalled();view.unmount();query.clear();
});
