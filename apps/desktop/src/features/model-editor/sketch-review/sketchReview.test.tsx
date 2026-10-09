import { afterEach,expect,it,vi } from "vitest";
import { act,cleanup,fireEvent,render,renderHook,screen } from "@testing-library/react";
import type { SketchAnalysis } from "../../../lib/sketchAnalysis";
import { createStarterSketch,type SketchOperation } from "../../../lib/sketchDocument";
import { useSketchBuildReview } from "./useSketchBuildReview";
import { SketchBuildReview } from "./SketchBuildReview";
const mocks=vi.hoisted(()=>({analyze:vi.fn()}));vi.mock("../../../lib/sketchAnalysis",()=>({analyzeSketch:mocks.analyze,sketchAnalysisAvailable:()=>true}));
afterEach(()=>{cleanup();vi.useRealTimers();vi.clearAllMocks();});
function draft(){const doc=JSON.parse(createStarterSketch());doc.features[0].operation.constraints=[{id:"horizontal",kind:"horizontal",lineId:"line_ab"}];return {...doc,features:doc.features as {id:string;name:string;operation:SketchOperation}[]};}
it("blocks changed constrained shapes and ignores a late response after a draft change",async()=>{
 vi.useFakeTimers();const document=draft(),source=JSON.stringify(document),solved=document.features[0].operation.points.map((point:SketchOperation["points"][number])=>({...point,xMm:point.xMm+10}));let resolve!:(value:Partial<SketchAnalysis>)=>void;mocks.analyze.mockImplementationOnce(()=>new Promise(done=>{resolve=done;})).mockResolvedValueOnce({status:"solved",solvedPoints:document.features[0].operation.points});
 const {result,rerender}=renderHook(({source})=>useSketchBuildReview(source,true),{initialProps:{source}});expect(result.current.pending).toBe(true);act(()=>vi.advanceTimersByTime(250));rerender({source:JSON.stringify({...document,revisionId:"new"})});await act(async()=>resolve({status:"solved",solvedPoints:solved}));expect(result.current.items).toHaveLength(0);expect(result.current.pending).toBe(true);await act(async()=>vi.advanceTimersByTime(250));expect(result.current.blocked).toBe(false);
});
it("keeps model identifiers and other features while applying an explicit shape choice",()=>{
 const document=draft(),source=JSON.stringify(document),operation=document.features[0].operation;const item={id:"sketch_1",name:"Profile",analysis:{status:"solved",solvedPoints:operation.points.map((point:SketchOperation["points"][number])=>({...point,xMm:point.xMm+10}))}} as {id:string;name:string;analysis:SketchAnalysis};const change=vi.fn();render(<SketchBuildReview source={source} items={[item]} pending={false} error={false} disabled={false} onChange={change}/>);
 fireEvent.click(screen.getByRole("button",{name:"Use drawn profile and remove constraints"}));let next=JSON.parse(change.mock.calls.at(-1)![0]);expect(next.features[0].operation.points).toEqual(operation.points);expect(next.features[0].operation.constraints).toEqual([]);expect(next.features[1]).toEqual(document.features[1]);expect(JSON.parse(source).features[0].operation.constraints).toHaveLength(1);
 fireEvent.click(screen.getByRole("button",{name:"Use solved coordinates"}));next=JSON.parse(change.mock.calls.at(-1)![0]);expect(next.features[0].operation.points).toEqual(item.analysis.solvedPoints);expect(next.features[0].operation.constraints).toHaveLength(1);
});
