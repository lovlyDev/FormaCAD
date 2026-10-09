import { afterEach,expect,test,vi } from "vitest";
import { act,cleanup,fireEvent,render,screen } from "@testing-library/react";
import { useState } from "react";
import { Sketch2dEditor } from "../Sketch2dEditor";
import { createStarterSketch,readSketchOperation,type SketchOperation } from "../../lib/sketchDocument";
const mocks=vi.hoisted(()=>({analyze:vi.fn()}));
vi.mock("../../lib/sketchAnalysis",()=>({analyzeSketch:mocks.analyze,sketchAnalysisAvailable:()=>true}));
afterEach(()=>{cleanup();vi.useRealTimers();vi.clearAllMocks();});
test("solved preview leaves draft unchanged until explicitly applied and restores dragging",async()=>{
 vi.useFakeTimers();const initial=readSketchOperation(JSON.parse(createStarterSketch()).features[0].operation)!;
 const solved=initial.points.map(p=>({...p,xMm:p.xMm+5}));
 mocks.analyze.mockResolvedValue({status:"solved",degreesOfFreedom:4,redundantEquations:0,holeCount:0,profileAreaMm2:800,solvedPoints:solved,constraints:[],maxResidualMm:0,errorCode:null});
 function Harness(){const [operation,setOperation]=useState<SketchOperation>(initial);return <><Sketch2dEditor operation={operation} disabled={false} onChange={setOperation}/><output data-testid="draft">{JSON.stringify(operation)}</output></>;}
 render(<Harness/>);fireEvent.click(screen.getByText("Edit 2D sketch"));await act(async()=>vi.advanceTimersByTime(250));
 fireEvent.click(screen.getByLabelText("Show solved sketch"));
 expect(JSON.parse(screen.getByTestId("draft").textContent!).points).toEqual(initial.points);
 const point=screen.getByLabelText("Sketch point point_a");expect(point).toHaveAttribute("cx",String(solved[0].xMm));expect(point).toHaveAttribute("tabindex","-1");
 fireEvent.click(screen.getByRole("button",{name:"Use solved coordinates"}));
 expect(JSON.parse(screen.getByTestId("draft").textContent!).points).toEqual(solved);expect(screen.queryByLabelText("Show solved sketch")).not.toBeInTheDocument();
 expect(point).toHaveAttribute("tabindex","0");
});
