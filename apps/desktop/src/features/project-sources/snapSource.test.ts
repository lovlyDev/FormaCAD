import {afterEach,expect,it,vi} from "vitest";
import {Group} from "three";
import {captureSceneRead,rememberSceneRead,displayedSceneRead} from "./sceneRead";
import {useWorkspace} from "../../stores/workspace";
import {pairTestContext} from "../pair-measurements/pairTestContext";
import {resolveSnap} from "../snapping/resolveSnap";
import {captureDisplayedSnapSource} from "../snapping/snapRequest";
import {useSnapSettings} from "../snapping/cursor/snapSettings";
import {useTransformViewport} from "../transform/viewport/transformStore";
import type {ExactSnapResult} from "../snapping/snapApi";
import type {SnapCandidate} from "../snapping/snapCandidate";
import {edgeSnapActions} from "../pie-menus/edgeSnapActions";
import {snapPie} from "../pie-menus/viewportPies";
const mocks=vi.hoisted(()=>({exact:vi.fn()}));
vi.mock("../snapping/snapApi",()=>({exactSnap:mocks.exact}));
vi.mock("../../lib/api",()=>({native:false}));
afterEach(()=>{useWorkspace.getState().setProject(null);useSnapSettings.setState({exact:true,pending:false,error:null});useTransformViewport.setState({cursorMm:[0,0,0]});vi.resetAllMocks();});
const point=():ExactSnapResult=>({pointMm:[1,2,3],normal:[0,0,1]});
function fixture(){
 const context=pairTestContext();
 useSnapSettings.setState({exact:true,pending:false,error:null});
 const open=(id="first")=>useWorkspace.getState().openFolderProject(context.project,{sourceId:id,commitId:"commit",taskScopeId:`task-${id}`});open();
 const candidate:SnapCandidate={kind:"face",bodyId:"body",pointMm:[9,9,9],query:{kind:"face",reference:context.face.topologyRef!,rayOriginMm:[0,0,50],rayDirection:[0,0,-1]}};
 const edge={bodyId:"body",edgeOrdinal:0,revisionId:context.project.currentRevision!,sceneToken:context.sceneToken,topologyRef:{schemaVersion:1 as const,kind:"edge" as const,ownerFeatureId:"pad",role:"box-edge:x:ymin:zmin" as const,occurrencePath:[]}};
 return{context,open,candidate,edge};
}
it("routes exact snap to the captured folder and rechecks after Promise settlement",async()=>{
 const{open,candidate}=fixture();mocks.exact.mockResolvedValue(point());const result=await resolveSnap(candidate);
 expect(mocks.exact.mock.calls[0][2]).toEqual({sourceId:"first",commitId:"commit",taskScopeId:"task-first"});
 expect(result?.exact).toBe(true);expect(result?.isCurrent()).toBe(true);open("second");expect(result?.isCurrent()).toBe(false);
});
it.each(["resolve","reject"] as const)("drops old %s after equal-ID reopen and clears stale pending/error",async mode=>{
 const{open,candidate}=fixture();let done!:(value:ExactSnapResult)=>void,fail!:(reason:Error)=>void;
 mocks.exact.mockImplementation(()=>new Promise<ExactSnapResult>((resolve,reject)=>{done=resolve;fail=reject;}));
 const pending=resolveSnap(candidate);expect(useSnapSettings.getState().pending).toBe(true);open();
 expect(useSnapSettings.getState().pending).toBe(false);if(mode==="resolve")done(point());else fail(Error("old failure"));
 expect(await pending).toBeNull();expect(useSnapSettings.getState().error).toBeNull();
});
it("does not accept another project's matching revision",async()=>{
 const{context,candidate}=fixture();let done!:(value:ExactSnapResult)=>void;mocks.exact.mockImplementation(()=>new Promise<ExactSnapResult>(resolve=>{done=resolve;}));
 const pending=resolveSnap(candidate);useWorkspace.getState().openManagedProject({...context.project,id:"other"});done(point());expect(await pending).toBeNull();
});
it("keeps the original in-flight commit across compatible metadata and uses fresh commit next time",async()=>{
 const{context,candidate}=fixture(),binding=useWorkspace.getState().sourceBinding;if(binding?.kind!=="folder")throw Error("fixture");
 let done!:(value:ExactSnapResult)=>void;mocks.exact.mockImplementationOnce(()=>new Promise<ExactSnapResult>(resolve=>{done=resolve;})).mockResolvedValueOnce(point());
 const pending=resolveSnap(candidate);expect(useWorkspace.getState().publishFolderSnapshot({...context.project,name:"Metadata"},binding,{...binding.source,commitId:"metadata"})).toBe(true);
 done(point());expect((await pending)?.exact).toBe(true);expect(mocks.exact.mock.calls[0][2].commitId).toBe("commit");
 await resolveSnap(candidate);expect(mocks.exact.mock.calls[1][2].commitId).toBe("metadata");
});
it("old failure/finally cannot overwrite the newer request's state",async()=>{
 const{candidate}=fixture();let fail!:(reason:Error)=>void,done!:(value:ExactSnapResult)=>void;
 mocks.exact.mockImplementationOnce(()=>new Promise<ExactSnapResult>((_,reject)=>{fail=reject;})).mockImplementationOnce(()=>new Promise<ExactSnapResult>(resolve=>{done=resolve;}));
 const first=resolveSnap(candidate),second=resolveSnap(candidate);fail(Error("old failure"));expect(await first).toBeNull();
 expect(useSnapSettings.getState().pending).toBe(true);expect(useSnapSettings.getState().error).toBeNull();done(point());expect((await second)?.exact).toBe(true);expect(useSnapSettings.getState().pending).toBe(false);
});
it("drops a result when the displayed scene unmounts",async()=>{
 const{candidate}=fixture();let active=true,done!:(value:ExactSnapResult)=>void;mocks.exact.mockImplementation(()=>new Promise<ExactSnapResult>(resolve=>{done=resolve;}));
 const pending=resolveSnap(candidate,()=>active);active=false;done(point());expect(await pending).toBeNull();expect(useSnapSettings.getState().pending).toBe(false);
});
it("clears a displayed failure when its source closes",async()=>{
 const{open,candidate}=fixture();mocks.exact.mockRejectedValue(Error("worker unavailable"));
 const result=await resolveSnap(candidate);expect(result?.exact).toBe(false);expect(useSnapSettings.getState().error).toBeInstanceOf(Error);
 open("second");expect(useSnapSettings.getState().error).toBeNull();expect(result?.isCurrent()).toBe(false);
});
it("an approximate placement supersedes an older exact request",async()=>{
 const{candidate}=fixture();let done!:(value:ExactSnapResult)=>void;mocks.exact.mockImplementation(()=>new Promise<ExactSnapResult>(resolve=>{done=resolve;}));
 const first=resolveSnap(candidate);useSnapSettings.setState({exact:false});const latest=await resolveSnap(candidate);
 expect(latest?.exact).toBe(false);expect(useSnapSettings.getState().pending).toBe(false);done(point());expect(await first).toBeNull();expect(latest?.isCurrent()).toBe(true);
});
it("a stale pie never retargets a newly opened equal-ID folder",async()=>{
 const{open,edge}=fixture();useWorkspace.getState().setSelectedEdge(edge);const menu=edgeSnapActions();expect(menu.ready).toBe(true);
 open("second");useWorkspace.getState().setSelectedEdge(edge);await menu.run("edgeMidpoint");expect(mocks.exact).not.toHaveBeenCalled();
});
it("pie snap uses its original source and cannot move the new source cursor",async()=>{
 const{open,edge}=fixture();useWorkspace.getState().setSelectedEdge(edge);let done!:(value:ExactSnapResult)=>void;
 mocks.exact.mockImplementation(()=>new Promise<ExactSnapResult>(resolve=>{done=resolve;}));const pending=edgeSnapActions().run("edgeMidpoint");
 expect(mocks.exact.mock.calls[0][2].sourceId).toBe("first");open("second");useTransformViewport.getState().setCursor([5,5,5]);done({pointMm:[1,2,3],normal:null});await pending;
 expect(useTransformViewport.getState().cursorMm).toEqual([5,5,5]);expect(useSnapSettings.getState().pending).toBe(false);
});

it("world-origin placement supersedes a pending exact placement",async()=>{
 const{candidate}=fixture();let done!:(value:ExactSnapResult)=>void;mocks.exact.mockImplementation(()=>new Promise<ExactSnapResult>(resolve=>{done=resolve;}));
 const pending=resolveSnap(candidate);useTransformViewport.getState().setCursor([5,5,5]);snapPie().items.find(item=>item.id==="origin")!.run();
 expect(useSnapSettings.getState().pending).toBe(false);done(point());expect(await pending).toBeNull();expect(useTransformViewport.getState().cursorMm).toEqual([0,0,0]);
});

it("does not replace a host stale-source refusal with a mesh estimate",async()=>{
 const{candidate}=fixture();mocks.exact.mockRejectedValue(Error("PROJECT_SNAPSHOT_CHANGED"));
 expect(await resolveSnap(candidate)).toBeNull();expect(useSnapSettings.getState().error).toBeInstanceOf(Error);expect(useSnapSettings.getState().pending).toBe(false);
});

it("an old displayed object never binds itself to the already-updated global source",async()=>{
 const{context,open,candidate}=fixture(),binding=useWorkspace.getState().sourceBinding;const object=new Group();
 rememberSceneRead(object,captureSceneRead(context.project,binding));const displayed=displayedSceneRead(object)!;open("second");
 expect(displayed.isCurrent()).toBe(false);expect(await resolveSnap(candidate,()=>displayed.isCurrent())).toBeNull();expect(mocks.exact).not.toHaveBeenCalled();
});
it("displayed source identity never enters userData or serialized object extras",()=>{
 const{context}=fixture(),binding=useWorkspace.getState().sourceBinding;if(!binding)throw Error("fixture");const object=new Group();
 rememberSceneRead(object,captureSceneRead(context.project,binding));expect(displayedSceneRead(object)?.isCurrent()).toBe(true);
 expect(object.userData).toEqual({});expect(JSON.stringify(object.toJSON())).not.toContain(binding.openingId);
 expect(displayedSceneRead(object.clone())).toBeUndefined();
});
it("same-opening source geometry edits invalidate the displayed original snapshot",async()=>{
 const{context,candidate}=fixture(),binding=useWorkspace.getState().sourceBinding;if(binding?.kind!=="folder")throw Error("fixture");const object=new Group();
 rememberSceneRead(object,captureSceneRead(context.project,binding));const displayed=displayedSceneRead(object)!;
 expect(useWorkspace.getState().publishFolderSnapshot({...context.project,files:context.project.files.map(file=>({...file,sha256:"b".repeat(64)}))},binding,{...binding.source,commitId:"geometry"})).toBe(true);
 expect(displayed.isCurrent()).toBe(false);expect(await resolveSnap(candidate,()=>displayed.isCurrent())).toBeNull();expect(mocks.exact).not.toHaveBeenCalled();
});

it("an unregistered folder object cannot borrow the active folder read context",()=>{
 fixture();expect(captureDisplayedSnapSource(new Group())).toBeNull();
});
it("a new menu on an old displayed object never borrows the updated source",async()=>{
 const{context,open,edge}=fixture(),binding=useWorkspace.getState().sourceBinding,object=new Group();object.uuid=edge.sceneToken;
 rememberSceneRead(object,captureSceneRead(context.project,binding));open("second");useWorkspace.getState().setSelectedEdge(edge);
 const menu=edgeSnapActions(object);expect(menu.ready).toBe(false);await menu.run("edgeMidpoint");expect(mocks.exact).not.toHaveBeenCalled();
});
it("a retired menu world-origin action cannot move another source cursor",()=>{
 const{open}=fixture();const menu=snapPie();open("second");useTransformViewport.getState().setCursor([5,5,5]);menu.items.find(item=>item.id==="origin")!.run();
 expect(useTransformViewport.getState().cursorMm).toEqual([5,5,5]);
});
