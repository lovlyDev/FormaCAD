import {afterEach,expect,it,vi} from "vitest";
import {act,cleanup,fireEvent,render,screen} from "@testing-library/react";
import {Group} from "three";
import {newProject,useWorkspace} from "../../stores/workspace";
import {CandidateReview} from "../agents/review/CandidateReview";
const mocks=vi.hoisted(()=>({preview:vi.fn(),baseline:vi.fn()}));
vi.mock("../model-preview/useModelPreview",()=>({useModelPreview:mocks.preview}));
vi.mock("../agents/review/useSavedBaseline",()=>({useSavedBaseline:mocks.baseline}));
vi.mock("../model-preview/PreviewCanvas",()=>({PreviewCanvas:()=>null}));
afterEach(()=>{cleanup();useWorkspace.getState().setProject(null);vi.resetAllMocks();});
function fixture(){
 const base=newProject("Review","blank","mm","codex");
 useWorkspace.getState().openFolderProject(base,{sourceId:"first",commitId:"commit",taskScopeId:"task"});
 mocks.preview.mockReturnValue({available:true,pending:false,waiting:false,preview:{object:new Group(),metrics:{volumeMm3:10,areaMm2:12,boundsMm:[1,2,5]}},error:null});
 mocks.baseline.mockReturnValue({object:null,error:false,pending:false});
 return{base,source:"{}",sourceBinding:useWorkspace.getState().sourceBinding};
}
it("routes both candidate and baseline to the captured opening",()=>{
 const review=fixture();render(<CandidateReview review={review} onDecision={()=>{}}/>);
 expect(mocks.preview.mock.calls[0][5]).toBe(review.sourceBinding);
 expect(mocks.baseline).toHaveBeenCalledWith(review.base,review.sourceBinding);
});
it("blocks acceptance after another folder opens with identical project data",async()=>{
 const review=fixture(),decision=vi.fn();render(<CandidateReview review={review} onDecision={decision}/>);
 expect(screen.getByRole("button",{name:"Accept candidate"})).toBeEnabled();
 await act(async()=>useWorkspace.getState().openFolderProject(review.base,{sourceId:"second",commitId:"commit",taskScopeId:"other-task"}));
 expect(screen.getByRole("button",{name:"Accept candidate"})).toBeDisabled();
 expect(mocks.baseline).toHaveBeenLastCalledWith(review.base,null);
 fireEvent.click(screen.getByRole("button",{name:"Accept candidate"}));expect(decision).not.toHaveBeenCalled();
});
it("does not turn a missing captured opening into current managed access",()=>{
 const review=fixture();render(<CandidateReview review={{...review,sourceBinding:null}} onDecision={()=>{}}/>);
 expect(screen.getByRole("button",{name:"Accept candidate"})).toBeDisabled();
 expect(mocks.preview.mock.calls[0][5]).toBeNull();
});
it("invalidates a strict candidate review when its original folder commit changes",async()=>{
 const review=fixture(),binding=review.sourceBinding;if(binding?.kind!=="folder")throw Error("fixture");
 render(<CandidateReview review={review} onDecision={()=>{}}/>);
 expect(screen.getByRole("button",{name:"Accept candidate"})).toBeEnabled();
 await act(async()=>{expect(useWorkspace.getState().publishFolderSnapshot({...review.base,name:"Metadata changed"},binding,{...binding.source,commitId:"metadata"})).toBe(true);});
 expect(screen.getByRole("button",{name:"Accept candidate"})).toBeDisabled();
 expect(mocks.baseline).toHaveBeenLastCalledWith(review.base,null);
});
