import {afterEach,expect,it,vi} from "vitest";
import {act,cleanup,render,screen,waitFor} from "@testing-library/react";
import {QueryClient,QueryClientProvider} from "@tanstack/react-query";
import {Group} from "three";
import {ModelInspector} from "../../app/ModelInspector";
import {newProject} from "../../stores/workspace";
import {defaults} from "../../types";
import {inspectModel} from "../../lib/model";
import type {ExactModelProperties} from "../../lib/modelInspection";
import {folderSource} from "./sourceBinding";
const mocks=vi.hoisted(()=>({inspect:vi.fn()}));
vi.mock("../../lib/api",()=>({native:true}));
vi.mock("../../lib/modelInspection",()=>({inspectSavedModel:mocks.inspect}));
afterEach(()=>{cleanup();vi.resetAllMocks();});
function fixture(){
 const project=newProject("Source properties","blank","mm","codex");project.currentRevision="head";
 project.revisions=[{id:"head",parent:null,createdAt:project.createdAt,prompt:"",parameters:defaults,source:"body.step"}];
 const props={project,revision:project.revisions[0],selectedLabel:null,selectedFace:null,selectedFaceAreaMm2:null,selectedEdge:null,selectedEdgeLengthMm:null,selectedEdgeRadiusMm:null,stats:inspectModel(new Group()),thickness:2,onEditParameters:()=>{}};
 const binding=()=>folderSource(project.id,{sourceId:"source",commitId:"commit",taskScopeId:"task"});
 const client=new QueryClient({defaultOptions:{queries:{retry:false,gcTime:0}}});
 const wrapper=({children}:{children:React.ReactNode})=><QueryClientProvider client={client}>{children}</QueryClientProvider>;
 return{props,binding,wrapper};
}
const report=(boundsMm:[number,number,number]):ExactModelProperties=>({boundsMm,volumeMm3:8000,areaMm2:2800,faceCount:6,edgeCount:12});
it("requests exact inspection from the captured host source",async()=>{
 const{props,binding,wrapper}=fixture(),source=binding();mocks.inspect.mockResolvedValue(report([40,20,10]));
 render(<ModelInspector {...props} sourceBinding={source}/>,{wrapper});
 await waitFor(()=>expect(mocks.inspect).toHaveBeenCalledTimes(1));
 expect(mocks.inspect).toHaveBeenCalledWith(props.project.id,"head",source.kind==="folder"?source.source:null);
 await screen.findByText("CAD dimensions (X × Y × Z)");
});
it("does not share cached or late properties across identical project/revision/source IDs after reopen",async()=>{
 const{props,binding,wrapper}=fixture();let resolve!:(value:ExactModelProperties)=>void;
 mocks.inspect.mockImplementationOnce(()=>new Promise<ExactModelProperties>(done=>{resolve=done;})).mockResolvedValueOnce(report([2,3,4]));
 const view=render(<ModelInspector {...props} sourceBinding={binding()}/>,{wrapper});
 await waitFor(()=>expect(mocks.inspect).toHaveBeenCalledTimes(1));view.rerender(<ModelInspector {...props} sourceBinding={binding()}/>);
 await waitFor(()=>expect(screen.getByText("CAD dimensions (X × Y × Z)").closest("div")).toHaveTextContent("2.00 × 3.00 × 4.00"));
 await act(async()=>resolve(report([40,20,10])));
 expect(screen.getByText("CAD dimensions (X × Y × Z)").closest("div")).toHaveTextContent("2.00 × 3.00 × 4.00");
 expect(mocks.inspect).toHaveBeenCalledTimes(2);
});
it("never makes a managed inspection call for missing or mismatched explicit binding",async()=>{
 const{props,wrapper}=fixture();const source=folderSource("other",{sourceId:"source",commitId:"commit",taskScopeId:"task"});
 const view=render(<ModelInspector {...props} sourceBinding={source}/>,{wrapper});
 await act(async()=>{});view.rerender(<ModelInspector {...props} sourceBinding={null}/>);await act(async()=>{});
 expect(mocks.inspect).not.toHaveBeenCalled();expect(screen.queryByText("CAD dimensions (X × Y × Z)")).toBeNull();
});
