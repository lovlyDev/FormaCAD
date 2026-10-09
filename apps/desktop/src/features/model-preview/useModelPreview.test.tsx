import { afterEach,describe,expect,it,vi } from "vitest";
import { act,cleanup,renderHook } from "@testing-library/react";
import { Group } from "three";
import { useModelPreview } from "./useModelPreview";
import type { ModelPreview } from "./previewApi";
const mocks=vi.hoisted(()=>({preview:vi.fn(),cancel:vi.fn().mockResolvedValue(undefined),dispose:vi.fn()}));
vi.mock("./previewApi",()=>({previewModel:mocks.preview,cancelModelPreview:mocks.cancel,modelPreviewAvailable:()=>true}));vi.mock("../../lib/model",()=>({disposeModel:mocks.dispose}));
const report=():ModelPreview=>({object:new Group(),metrics:{volumeMm3:1000,areaMm2:500,faceCount:6,edgeCount:12,boundsMm:[10,20,5]}});
afterEach(()=>{cleanup();vi.useRealTimers();vi.clearAllMocks();});
describe("model preview scheduling",()=>{
 it("serializes requests and discards the stale model before building the latest draft",async()=>{
  vi.useFakeTimers();let first!:(value:ModelPreview)=>void,second!:(value:ModelPreview)=>void;mocks.preview.mockImplementationOnce(()=>new Promise<ModelPreview>(resolve=>{first=resolve;})).mockImplementationOnce(()=>new Promise<ModelPreview>(resolve=>{second=resolve;}));
  const {result,rerender}=renderHook(({source})=>useModelPreview("project",source,"revision",true,0),{initialProps:{source:"first"}});
  act(()=>vi.advanceTimersByTime(500));expect(mocks.preview).toHaveBeenCalledTimes(1);rerender({source:"latest"});act(()=>vi.advanceTimersByTime(1500));expect(mocks.preview).toHaveBeenCalledTimes(1);
  const stale=report();await act(async()=>first(stale));expect(mocks.dispose).toHaveBeenCalledWith(stale.object);expect(result.current.preview).toBeNull();
  act(()=>vi.advanceTimersByTime(500));expect(mocks.preview).toHaveBeenCalledTimes(2);const fresh=report();await act(async()=>second(fresh));expect(result.current.preview).toBe(fresh);expect(result.current.waiting).toBe(false);
 });
 it("cancels its own request on disable and disposes a late response",async()=>{
  vi.useFakeTimers();let resolve!:(value:ModelPreview)=>void;mocks.preview.mockImplementationOnce(()=>new Promise<ModelPreview>(done=>{resolve=done;}));
  const {result,rerender}=renderHook(({enabled})=>useModelPreview("project","source",null,enabled,0),{initialProps:{enabled:true}});act(()=>vi.advanceTimersByTime(500));rerender({enabled:false});expect(mocks.cancel).toHaveBeenCalledWith("project");
  const late=report();await act(async()=>resolve(late));expect(mocks.dispose).toHaveBeenCalledWith(late.object);expect(result.current.preview).toBeNull();expect(result.current.pending).toBe(false);
 });
});

it("retains previous geometry while waiting and replaces it only with the latest result",async()=>{
 vi.useFakeTimers();let resolve!:(value:ModelPreview)=>void;const initial=report();mocks.preview.mockResolvedValueOnce(initial).mockImplementationOnce(()=>new Promise<ModelPreview>(done=>{resolve=done;}));
 const {result,rerender}=renderHook(({source})=>useModelPreview("project",source,null,true,0),{initialProps:{source:"old"}});await act(async()=>vi.advanceTimersByTime(500));expect(result.current.preview).toBe(initial);
 rerender({source:"new"});expect(result.current.preview).toBe(initial);expect(result.current.waiting).toBe(true);act(()=>vi.advanceTimersByTime(500));expect(mocks.dispose).not.toHaveBeenCalledWith(initial.object);
 const next=report();await act(async()=>resolve(next));expect(result.current.preview).toBe(next);expect(result.current.waiting).toBe(false);expect(mocks.dispose).toHaveBeenCalledWith(initial.object);
});
