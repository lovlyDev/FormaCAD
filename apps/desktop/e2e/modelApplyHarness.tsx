/** Actual App and IPC boundary mock; native geometry comes from the isolated real worker fixture. */
export async function mountModelApplyHarness(fixture: { source: string; candidate: string; assets: Record<string, string>; metadata: Record<string, {size:number;sha256:string}>; metrics: Record<string, any>; rejection: any }) {
  const callbacks = new Map<number, (...args: any[]) => void>(), listeners = new Map<number, {event:string;handler:number}>(); let callback = 0, eventId = 0;
  const calls: {command:string;args:any}[] = [], projects: Record<string, any> = {};
  let state: any, releaseFile: ((value: string) => void) | null = null, failFile: ((cause: Error) => void) | null = null;
  let outcome: "commit" | "noop" | "error" = "commit", delayed = false;
  let rejectPlan: ((cause: Error) => void) | null = null;
  const invoke = async (command: string, args: any) => {
    calls.push({command,args});
    if (command === "plugin:event|listen") { listeners.set(++eventId,{event:args.event,handler:args.handler}); return eventId; }
    if (command === "list_projects") return Object.values(projects);
    if (command === "interrupted_sessions" || command === "cad_task_status") return [];
    if (command === "detect_environment") return [{name:"OpenCascade kernel",available:true,detail:"isolated fixture"}];
    if (command === "acquire_project_access") return {projectId:args.projectId,mode:"write",ownerPid:1};
    if (command === "get_confirmation_settings") return {mode:"all",overrides:{}};
    if (command === "request_permission") return "grant";
    if (command === "history_status") return {canUndo:projects[args.projectId].revisions.length>1,canRedo:false,expectedRevision:projects[args.projectId].currentRevision};
    if (command === "inspect_model") return fixture.metrics[args.revisionId === "accepted" ? "candidate" : "base"];
    if (command === "section_model") return {sourceSha256:fixture.metadata[args.expectedRevision==="accepted"?"candidate.step":"base.step"].sha256,geometry:{plane:{originMm:args.originMm,normal:args.normal,deflectionMm:args.deflectionMm},totalLengthMm:0,curves:[]}};
    if (command === "read_project_file") {
      if (args.name === "candidate.glb" && delayed) return new Promise<string>((resolve,reject)=>{releaseFile=resolve;failFile=reject;});
      return fixture.assets[args.name];
    }
    if (command === "apply_program") {
      if (outcome === "error") throw JSON.stringify({code:fixture.rejection.code,message:fixture.rejection.detail});
      const current = projects[args.projectId]; if(outcome === "noop") return current;
      const document = JSON.parse(args.program); document.revisionId="accepted";
      const next = {...current,currentRevision:"accepted",revisions:[...current.revisions,{...current.revisions[0],id:"accepted",parent:current.currentRevision,source:"candidate.step",preview:"candidate.glb",program:JSON.stringify(document)}],files:[...current.files,...["candidate.step","candidate.glb"].map(name=>({name,kind:"model",...fixture.metadata[name]}))]};
      projects[args.projectId]=next;return next;
    }
    if (command === "plan_model") return new Promise((_resolve,reject)=>{rejectPlan=reject;});
    if (command === "save_project") { projects[args.project.id]=args.project;return args.project; }
    if (command === "get_agent_settings") return {};
    if (command.startsWith("plugin:") || command === "set_ui_preferences" || command === "release_project_access" || command === "resolve_permission") return null;
    return null;
  };
  Object.assign(window, {isTauri:true,__TAURI_EVENT_PLUGIN_INTERNALS__:{unregisterListener:(_event:string,id:number)=>listeners.delete(id)},__TAURI_INTERNALS__:{metadata:{currentWindow:{label:"main"},currentWebview:{label:"main"}},invoke,
    transformCallback:(handler:(...args:any[])=>void)=>{callbacks.set(++callback,handler);return callback;},unregisterCallback:(id:number)=>callbacks.delete(id)}});
  const React = await import("react"), ReactDOM = await import("react-dom/client"), Router = await import("react-router-dom"), Query = await import("@tanstack/react-query"), Tooltip = await import("@radix-ui/react-tooltip");
  const {native}=await import("../src/lib/api");if(!native)throw Error("Actual App fixture imported API before native flags were set");
  const {useWorkspace,newProject} = await import("../src/stores/workspace"); state=useWorkspace;
  const project=newProject("Apply A","blank","mm","codex");project.id="A";project.currentRevision="base";
  project.revisions=[{id:"base",parent:null,createdAt:project.createdAt,prompt:"Baseline",parameters:{kind:"blank",width:1,depth:1,height:1,thickness:1,holeDiameter:0,holes:0},source:"base.step",preview:"base.glb",program:fixture.source}];
  project.files=["base.step","base.glb"].map(name=>({name,kind:"model",...fixture.metadata[name]}));
  projects.A=project;projects.B={...project,id:"B",name:"Apply B"};useWorkspace.getState().setProject(project);
  const {default:App}=await import("../src/app/App"), {applyTheme}=await import("../src/lib/theme");applyTheme("dark");
  await import("../src/styles.css");
  const root=ReactDOM.createRoot(document.getElementById("root")!);
  const Fiber=await import("@react-three/fiber"),THREE=await import("three");
  const {persist}=await import("../src/lib/persistence");
  let navigate: ReturnType<typeof Router.useNavigate>;
  function Navigation(){navigate=Router.useNavigate();return null;}
  root.render(<Query.QueryClientProvider client={new Query.QueryClient({defaultOptions:{queries:{retry:false}}})}><Tooltip.Provider><Router.MemoryRouter initialEntries={["/project/A"]}><Router.Routes><Router.Route path="/project/:projectId" element={<><Navigation/><App/></>}/></Router.Routes></Router.MemoryRouter></Tooltip.Provider></Query.QueryClientProvider>);
  Object.assign(window,{__applyHarness:{calls,project:()=>state.getState().project,setOutcome:(value:typeof outcome)=>{outcome=value;},setDelayed:(value:boolean)=>{delayed=value;},release:()=>releaseFile?.(fixture.assets["candidate.glb"]),fail:()=>failFile?.(Error("fixture preview unavailable")),filePending:()=>!!releaseFile,
    switchProject:()=>{state.getState().setProject(projects.B);navigate("/project/B");},theme:applyTheme,candidate:fixture.candidate,camera:()=>localStorage.getItem("forma.ui.project.A.view.main.camera"),
    emitRevision:(id="A")=>listeners.forEach((listener,key)=>{if(listener.event==="project://revision-created")callbacks.get(listener.handler)?.({event:listener.event,id:key,payload:id});}),
    listenerCount:()=>[...listeners.values()].filter(listener=>listener.event==="project://revision-created").length,
    inspect:()=>{
      const canvas=document.querySelector(".viewer-area canvas") as HTMLCanvasElement;
      const live=Fiber._roots.get(canvas)?.store.getState(); if(!live)return null;
      const body=live.scene.getObjectByName("body") as any;
      if(!body?.geometry)return null;
      return {uuid:body.uuid,vertices:body.geometry.attributes.position.count,clipping:body.material.clippingPlanes?.length??0,
        bounds:new THREE.Box3().setFromObject(body).getSize(new THREE.Vector3()).toArray(),camera:[...live.camera.position.toArray(),...live.camera.quaternion.toArray()].map(value=>Math.round(value*100000)/100000)};
    },
    selection:()=>state.getState().selected,
    section:()=>{persist("forma.ui.project.A.view.main.section.enabled","true");persist("forma.ui.project.A.view.main.section.offsetMm","5");},
    planPending:()=>!!rejectPlan, rejectPlan:()=>rejectPlan?.(Error("captured A agent failure")),
    unmount:()=>root.unmount()}});
}
