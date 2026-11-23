import {afterEach,expect,it,vi} from "vitest";
import {requestNativePermission,resolveNativePermission} from "../../lib/api";
import {capturePermissionContext,type FolderPermissionOptions} from "./permissionContract";
const mocks=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock("@tauri-apps/api/core",async original=>({...await original<typeof import("@tauri-apps/api/core")>(),invoke:mocks.invoke,isTauri:()=>true}));
afterEach(()=>vi.resetAllMocks());
const id="0fdca3bb-60ac-4014-99e9-28b8306687c7";
function options():FolderPermissionOptions{return{source:{sourceId:"source",commitId:"commit",taskScopeId:"task"},intent:{kind:"model",program:"original program",prompt:"Original",expectedRevision:null}};}
it("preserves the existing managed request and decision transport shapes",async()=>{
 mocks.invoke.mockResolvedValue("legacy-id");expect(await requestNativePermission("project","modify_project","detail")).toBe("legacy-id");
 expect(mocks.invoke).toHaveBeenCalledWith("request_permission",{projectId:"project",action:"modify_project",detail:"detail"});
 await resolveNativePermission("legacy-id",true);expect(mocks.invoke).toHaveBeenLastCalledWith("resolve_permission",{id:"legacy-id",allow:true});
});
it("routes a typed model intent to the original source and retains the host confirmation policy",async()=>{
 const folder=options();mocks.invoke.mockResolvedValue({id,requiresConfirmation:true});const ticket=await requestNativePermission("project","modify_project","detail",folder);
 expect(mocks.invoke).toHaveBeenCalledWith("request_permission",{projectId:"project",action:"modify_project",detail:"detail",sourceId:"source",expectedCommitId:"commit",folderIntent:folder.intent});
 expect(ticket).toEqual({id,requiresConfirmation:true,projectId:"project",source:folder.source});expect(Object.isFrozen(ticket)).toBe(true);expect(Object.isFrozen(ticket.source)).toBe(true);
 await resolveNativePermission(ticket.id,true,ticket);expect(mocks.invoke).toHaveBeenLastCalledWith("resolve_permission",{id,allow:true,projectId:"project",sourceId:"source",expectedCommitId:"commit"});
});
it("captures request source and intent before awaiting the native reply",async()=>{
 const source={sourceId:"source",commitId:"commit",taskScopeId:"task"},intent={kind:"model" as const,program:"original program",prompt:"Original",expectedRevision:null};
 let finish!:(value:unknown)=>void;mocks.invoke.mockImplementation(()=>new Promise(resolve=>{finish=resolve;}));
 const pending=requestNativePermission("project","modify_project","detail",{source,intent});source.sourceId="foreign";source.commitId="new";intent.program="replacement";
 finish({id,requiresConfirmation:true});const ticket=await pending;
 expect(ticket.source.sourceId).toBe("source");expect(ticket.source.commitId).toBe("commit");expect(mocks.invoke.mock.calls[0][1].folderIntent.program).toBe("original program");
});
it("uses the same route for a typed history decision and does not manufacture automatic approval",async()=>{
 const folder:FolderPermissionOptions={source:options().source,intent:{kind:"history",direction:"undo",expectedRevision:"revision"}};
 mocks.invoke.mockResolvedValue({id,requiresConfirmation:false});const ticket=await requestNativePermission("project","modify_project","detail",folder);
 expect(ticket.requiresConfirmation).toBe(false);expect(mocks.invoke).toHaveBeenCalledTimes(1);
 expect(mocks.invoke.mock.calls[0][1].folderIntent).toEqual(folder.intent);expect(mocks.invoke.mock.calls[0][1]).not.toHaveProperty("allow");
});
it("captures nested CAD commands before awaiting the permission reply",async()=>{
 const commands=[{command:"set_literal",featureId:"profile",field:"width",valueMm:80}];let finish!:(value:unknown)=>void;
 mocks.invoke.mockImplementation(()=>new Promise(resolve=>{finish=resolve;}));
 const pending=requestNativePermission("project","modify_project","detail",{source:options().source,intent:{kind:"commands",commands,prompt:"Width",expectedRevision:"revision"}});
 commands[0].valueMm=999;finish({id,requiresConfirmation:true});await pending;
 expect(mocks.invoke.mock.calls[0][1].folderIntent.commands[0].valueMm).toBe(80);
});
it("retains the original nested ReviewPlan with its model request",async()=>{
 const reviewPlan={assumptions:[],dimensions:[],affectedBodyIds:["body"],expectedChecks:[{type:"volume" as const,valueMm3:8000,toleranceMm3:0}]};let finish!:(value:unknown)=>void;
 mocks.invoke.mockImplementation(()=>new Promise(resolve=>{finish=resolve;}));
 const pending=requestNativePermission("project","modify_project","detail",{source:options().source,intent:{kind:"model",program:"program",prompt:"Model",expectedRevision:null,reviewPlan}});
 reviewPlan.expectedChecks[0].valueMm3=1;finish({id,requiresConfirmation:true});await pending;
 expect(mocks.invoke.mock.calls[0][1].folderIntent.reviewPlan.expectedChecks[0].valueMm3).toBe(8000);
});
it.each([{id:"invalid",requiresConfirmation:true},{id,requiresConfirmation:"false"},"managed-id"])("refuses malformed folder reply without a managed fallback: %j",async value=>{
 mocks.invoke.mockResolvedValue(value);await expect(requestNativePermission("project","modify_project","detail",options())).rejects.toThrow("PROJECT_SNAPSHOT_CHANGED");expect(mocks.invoke).toHaveBeenCalledTimes(1);
});
it("refuses explicit null or incomplete source contexts instead of managed requests",async()=>{
 await expect(requestNativePermission("project","modify_project","detail",null as never)).rejects.toThrow("PROJECT_SNAPSHOT_CHANGED");
 await expect(resolveNativePermission(id,true,null as never)).rejects.toThrow("PROJECT_SNAPSHOT_CHANGED");
 expect(()=>capturePermissionContext({projectId:"project",source:{sourceId:"source",commitId:"",taskScopeId:"task"}})).toThrow("PROJECT_SNAPSHOT_CHANGED");expect(mocks.invoke).not.toHaveBeenCalled();
});
