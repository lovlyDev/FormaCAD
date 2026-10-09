import { act, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { expect, it, vi } from "vitest";
import { useRevisionEvents } from "./useRevisionEvents";
import { newProject, useWorkspace } from "../../stores/workspace";
import type { Project } from "../../types";
const mock=vi.hoisted(()=>({listen:vi.fn(),list:vi.fn()}));
vi.mock("@tauri-apps/api/event",()=>({listen:mock.listen}));
vi.mock("../../lib/api",()=>({native:true,listProjects:mock.list,saveProject:vi.fn()}));

it.each(["workspace-and-cache", "cache-only"])("a delayed event cannot erase newer same-head metadata in %s",async route=>{
  const base=newProject("A","blank","mm","codex"),query=new QueryClient(),stop=vi.fn();
  base.id="A";base.currentRevision="base";
  let event!: (value:{payload:string})=>void,finish!: (projects:Project[])=>void;
  mock.listen.mockImplementation(async(_name,handler)=>{event=handler;return stop;});
  mock.list.mockImplementation(()=>new Promise(resolve=>{finish=resolve;}));
  useWorkspace.getState().setProject(base);query.setQueryData(["projects"],[base]);
  const view=renderHook(()=>useRevisionEvents(),{wrapper:({children})=><QueryClientProvider client={query}>{children}</QueryClientProvider>});
  await waitFor(()=>expect(event).toBeDefined());
  act(()=>event({payload:"A"}));
  const newer={...base,messages:[{id:"message",role:"assistant" as const,text:"saved after fetch started",createdAt:base.createdAt}]};
  act(()=>{if(route==="workspace-and-cache")useWorkspace.getState().setProject(newer);query.setQueryData(["projects"],[newer]);});
  const cachedNewer=query.getQueryData<Project[]>(["projects"])?.[0];
  await act(async()=>{finish([base]);await Promise.resolve();});
  expect(useWorkspace.getState().project).toBe(route==="workspace-and-cache"?newer:base);
  expect(query.getQueryData<Project[]>(["projects"])?.[0]).toBe(cachedNewer);
  expect(query.getQueryData<Project[]>(["projects"])?.[0]?.messages).toEqual(newer.messages);
  view.unmount();expect(stop).toHaveBeenCalledTimes(1);query.clear();
});

it("an unrelated project notification updates only its cache entry",async()=>{
  const active=newProject("A","blank","mm","codex"),other=newProject("B","blank","mm","codex"),query=new QueryClient();
  active.id="A";other.id="B";const latest={...other,currentRevision:"new"};
  let event!: (value:{payload:string})=>void;
  mock.listen.mockImplementation(async(_name,handler)=>{event=handler;return vi.fn();});
  mock.list.mockResolvedValue([{...active,name:"outdated active metadata"},latest]);
  useWorkspace.getState().setProject(active);query.setQueryData(["projects"],[active,other]);
  const view=renderHook(()=>useRevisionEvents(),{wrapper:({children})=><QueryClientProvider client={query}>{children}</QueryClientProvider>});
  await waitFor(()=>expect(event).toBeDefined());
  await act(async()=>{event({payload:"B"});await Promise.resolve();});
  expect(useWorkspace.getState().project).toBe(active);
  expect(query.getQueryData<Project[]>(["projects"])).toEqual([active,latest]);
  view.unmount();query.clear();
});
