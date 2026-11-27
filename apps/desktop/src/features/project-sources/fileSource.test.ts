import {beforeEach,expect,it,vi} from "vitest";
import {waitFor} from "@testing-library/react";
import {loadModel} from "../../lib/modelLoader";
import {newProject} from "../../stores/workspace";
const mock=vi.hoisted(()=>({read:vi.fn(),snapshot:vi.fn(),dispose:vi.fn()}));
vi.mock("../../lib/api",()=>({native:true,readProjectFile:mock.read,saveProject:vi.fn()}));
vi.mock("../project-catalog/api",()=>({readProjectSnapshot:mock.snapshot}));
vi.mock("../../lib/model",()=>({disposeModel:mock.dispose}));
const source={sourceId:"folder",commitId:"commit",taskScopeId:"group"};
// Renderer decoder fixture only; this triangle is not a native CAD acceptance model.
const stl="solid fixture\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid fixture\n";
const file={name:"source.stl",kind:"model" as const,size:new TextEncoder().encode(stl).length,sha256:"a".repeat(64),data:"managed-inline-must-not-be-used"};
const project=()=>({...newProject("File","blank","mm","codex"),id:"shared",files:[file]});
beforeEach(()=>{mock.read.mockReset().mockResolvedValue(`data:model/stl;base64,${btoa(stl)}`);mock.snapshot.mockReset();mock.dispose.mockReset();});
it("reads folder bytes even when an inline managed candidate exists and rechecks the original source after decoding",async()=>{
  mock.snapshot.mockResolvedValue(project());const object=await loadModel(file,"shared",source);
  expect(mock.read).toHaveBeenCalledWith("shared",file.name,source);expect(mock.snapshot).toHaveBeenCalledWith("shared",source);
  expect(object.children).toHaveLength(1);expect(mock.dispose).not.toHaveBeenCalled();
});
it("refuses metadata from an equal-ID source with a different seal and disposes the decoded object",async()=>{
  mock.snapshot.mockResolvedValue({...project(),files:[{...file,sha256:"b".repeat(64)}]});
  await expect(loadModel(file,"shared",source)).rejects.toThrow("PROJECT_SNAPSHOT_CHANGED");expect(mock.dispose).toHaveBeenCalledTimes(1);
});
it("refuses close during the final source recheck after the real decoder finishes",async()=>{
  let reject!:(cause:Error)=>void;mock.snapshot.mockImplementation(()=>new Promise((_resolve,no)=>{reject=no;}));
  const pending=loadModel(file,"shared",source);const result=expect(pending).rejects.toThrow("PROJECT_SNAPSHOT_CHANGED");
  await waitFor(()=>expect(mock.snapshot).toHaveBeenCalledTimes(1));reject(new Error("PROJECT_SNAPSHOT_CHANGED"));await result;
  expect(mock.dispose).toHaveBeenCalledTimes(1);expect(mock.dispose.mock.calls[0][0].children).toHaveLength(1);
});
