import {act,renderHook,waitFor} from "@testing-library/react";
import {expect,it,vi} from "vitest";
import {useProjectAccess} from "../project-access/useProjectAccess";
import type {ProjectAccess} from "../project-access/projectAccessApi";
const mock=vi.hoisted(()=>({acquire:vi.fn(),retain:vi.fn(()=>vi.fn())}));
vi.mock("../../lib/api",()=>({native:true}));
vi.mock("../project-access/accessLifecycle",()=>({acquireAccess:mock.acquire,retainAccess:mock.retain}));
it("does not reuse a previous opening's write status while checking an equal-ID reopen",async()=>{
  let finish!:(status:ProjectAccess)=>void;const error=vi.fn();
  mock.acquire.mockResolvedValueOnce({projectId:"shared",mode:"write",ownerPid:1}).mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));
  const view=renderHook(({scope})=>useProjectAccess("shared",error,scope),{initialProps:{scope:"first-opening"}});
  await waitFor(()=>expect(view.result.current.writable).toBe(true));
  view.rerender({scope:"reopened"});expect(view.result.current.checking).toBe(true);expect(view.result.current.writable).toBe(false);expect(view.result.current.status).toBeUndefined();
  await act(async()=>{finish({projectId:"shared",mode:"read_only",ownerPid:2});await Promise.resolve();});
  await waitFor(()=>expect(view.result.current.checking).toBe(false));expect(view.result.current.writable).toBe(false);expect(view.result.current.status?.ownerPid).toBe(2);
  expect(mock.acquire).toHaveBeenCalledTimes(2);view.unmount();
});
it("keeps an absent managed project safe when a folder opening scope is supplied",()=>{
  mock.acquire.mockClear();const view=renderHook(()=>useProjectAccess(undefined,vi.fn(),"folder-opening"));
  expect(view.result.current.checking).toBe(false);expect(view.result.current.status).toBeUndefined();expect(mock.acquire).not.toHaveBeenCalled();view.unmount();
});
