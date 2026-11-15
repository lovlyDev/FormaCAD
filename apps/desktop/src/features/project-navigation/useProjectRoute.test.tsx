import {afterEach,beforeEach,it,expect,vi} from "vitest";
import {cleanup,render,screen,waitFor,fireEvent} from "@testing-library/react";
import {MemoryRouter,Routes,Route,useLocation,useNavigate} from "react-router-dom";
import {QueryClient,QueryClientProvider} from "@tanstack/react-query";
import type {Project} from "../../types";
import {useProjectRoute} from "./useProjectRoute";
const fake=vi.hoisted(()=>({project:null as Project|null,adopt:vi.fn(),fail:vi.fn(),read:vi.fn()}));
vi.mock("../../lib/api",()=>({listProjects:fake.read}));
vi.mock("../../stores/workspace",()=>({useWorkspace:Object.assign((selector:(state:unknown)=>unknown)=>selector({setProject:fake.adopt,setError:fake.fail}),{getState:()=>({project:fake.project})})}));
const empty:Project[]=[];
function Harness(){useProjectRoute(empty);const location=useLocation(),nav=useNavigate();return <><span>{location.pathname}</span><button onClick={()=>nav("/project/b")}>next</button></>;}
function start(){return render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><MemoryRouter initialEntries={["/project/a"]}><Routes><Route path="*" element={<Harness/>}/><Route path="/project/:projectId" element={<Harness/>}/></Routes></MemoryRouter></QueryClientProvider>);}
beforeEach(()=>{vi.clearAllMocks();fake.project=null;});
afterEach(cleanup);
it("freshly resolves a project missing from the cached list instead of redirecting home",async()=>{
 const project={id:"a"} as Project;fake.read.mockResolvedValue([project]);start();
 await waitFor(()=>expect(fake.adopt).toHaveBeenCalledWith(project));expect(screen.getByText("/project/a")).toBeTruthy();expect(fake.read).toHaveBeenCalledTimes(1);
});
it("ignores a delayed result after the route changes",async()=>{
 let finish!:(projects:Project[])=>void;fake.read.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;})).mockResolvedValue([{id:"b"}]);start();
 await waitFor(()=>expect(fake.read).toHaveBeenCalledTimes(1));fireEvent.click(screen.getByText("next"));
 await waitFor(()=>expect(fake.adopt).toHaveBeenCalledWith({id:"b"}));finish([{id:"a"} as Project]);await new Promise(resolve=>setTimeout(resolve,0));
 expect(fake.adopt).not.toHaveBeenCalledWith({id:"a"});
});
it("reports a failed authoritative read without retry or redirect",async()=>{
 fake.read.mockRejectedValue(new Error("read refused"));start();await waitFor(()=>expect(fake.fail).toHaveBeenCalled());expect(fake.read).toHaveBeenCalledTimes(1);expect(screen.getByText("/project/a")).toBeTruthy();
});
it("clears another project's editable scene before resolving the requested project",async()=>{
 fake.project={id:"previous"} as Project;fake.read.mockResolvedValue([{id:"a"}]);start();
 await waitFor(()=>expect(fake.adopt).toHaveBeenCalledWith({id:"a"}));expect(fake.adopt.mock.calls[0]).toEqual([null]);
});
