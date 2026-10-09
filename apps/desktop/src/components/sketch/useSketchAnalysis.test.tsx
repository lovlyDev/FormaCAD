import { afterEach,describe,expect,it,vi } from "vitest";
import { act,cleanup,renderHook } from "@testing-library/react";
import { useSketchAnalysis } from "./useSketchAnalysis";
import { createStarterSketch,readSketchOperation } from "../../lib/sketchDocument";
import type { SketchAnalysis } from "../../lib/sketchAnalysis";
const mocks=vi.hoisted(()=>({analyze:vi.fn(),available:vi.fn(()=>true)}));
vi.mock("../../lib/sketchAnalysis",()=>({analyzeSketch:mocks.analyze,sketchAnalysisAvailable:mocks.available}));
const report:SketchAnalysis={status:"solved",errorCode:null,degreesOfFreedom:4,redundantEquations:0,maxResidualMm:0,constraints:[],solvedPoints:[],loopCount:1,holeCount:0,profileAreaMm2:800};
const starter=()=>readSketchOperation(JSON.parse(createStarterSketch()).features[0].operation)!;
afterEach(()=>{cleanup();vi.useRealTimers();vi.clearAllMocks();});
describe("sketch analysis lifecycle",()=>{
 it("debounces edits and hides stale results until the current request resolves",async()=>{
  vi.useFakeTimers();let firstResolve!:(value:SketchAnalysis)=>void,secondResolve!:(value:SketchAnalysis)=>void;
  mocks.analyze.mockImplementationOnce(()=>new Promise<SketchAnalysis>(resolve=>{firstResolve=resolve;})).mockImplementationOnce(()=>new Promise<SketchAnalysis>(resolve=>{secondResolve=resolve;}));
  const first=starter(),second={...first,plane:"xz" as const};
  const {result,rerender}=renderHook(({op})=>useSketchAnalysis(op,[],"sketch"),{initialProps:{op:first}});
  act(()=>vi.advanceTimersByTime(200));expect(mocks.analyze).not.toHaveBeenCalled();
  act(()=>vi.advanceTimersByTime(50));expect(mocks.analyze).toHaveBeenCalledTimes(1);
  rerender({op:second});expect(result.current.analysis).toBeNull();
  await act(async()=>firstResolve(report));expect(result.current.analysis).toBeNull();
  act(()=>vi.advanceTimersByTime(250));expect(mocks.analyze).toHaveBeenCalledTimes(2);
  await act(async()=>secondResolve({...report,profileAreaMm2:900}));expect(result.current.analysis?.profileAreaMm2).toBe(900);expect(result.current.pending).toBe(false);
 });
 it("does not invoke native analysis in browser mode",()=>{
  vi.useFakeTimers();mocks.available.mockReturnValueOnce(false);
  const {result}=renderHook(()=>useSketchAnalysis(starter(),[],"sketch"));act(()=>vi.advanceTimersByTime(1000));
  expect(mocks.analyze).not.toHaveBeenCalled();expect(result.current.available).toBe(false);expect(result.current.pending).toBe(false);
 });
});
