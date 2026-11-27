import {renderHook} from "@testing-library/react";
import {expect,it,vi} from "vitest";
import {useWorkspaceSourceAccess} from "./useWorkspaceSourceAccess";
import {folderSource,managedSource} from "./sourceBinding";
import {newProject,useWorkspace} from "../../stores/workspace";
const mock=vi.hoisted(()=>({access:vi.fn((_id?:string,_onError?:(message:string)=>void)=>({writable:true,checking:false,status:{mode:"write"},retry:vi.fn()}))}));
vi.mock("../project-access/useProjectAccess",()=>({useProjectAccess:mock.access}));
it("does not acquire managed editing for a read-only folder sharing its project ID",()=>{
  const binding=folderSource("shared",{sourceId:"folder",commitId:"commit",taskScopeId:"group"});
  const view=renderHook(()=>useWorkspaceSourceAccess("shared",binding,vi.fn()));expect(mock.access.mock.lastCall?.[0]).toBeUndefined();
  expect(view.result.current.writable).toBe(false);expect(view.result.current.status).toBeUndefined();view.unmount();
});
it("retains existing managed access behavior",()=>{
  const view=renderHook(()=>useWorkspaceSourceAccess("shared",managedSource("shared"),vi.fn()));
  expect(mock.access.mock.lastCall?.[0]).toBe("shared");expect(view.result.current.writable).toBe(true);view.unmount();
});
it("does not report an old managed release error into an equal-ID folder view",()=>{
  const project={...newProject("Access","blank","mm","codex"),id:"shared"};useWorkspace.getState().setProject(project);
  const source=useWorkspace.getState().sourceBinding!,error=vi.fn();
  const view=renderHook(()=>useWorkspaceSourceAccess(project.id,source,error));const report=mock.access.mock.lastCall![1]!;
  useWorkspace.getState().setProject({...project,name:"metadata"});report("current opening error");expect(error).toHaveBeenCalledWith("current opening error");error.mockClear();
  useWorkspace.getState().openFolderProject(project,{sourceId:"folder",commitId:"commit",taskScopeId:"group"});report("late release error");expect(error).not.toHaveBeenCalled();
  view.unmount();useWorkspace.getState().setProject(null);
});
