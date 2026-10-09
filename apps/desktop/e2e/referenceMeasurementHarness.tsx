import { mountModelApplyHarness } from "./modelApplyHarness";

/** Reuses the actual App mount; measurement reports originate in the native fixture. */
export async function mountReferenceMeasurementHarness(fixture:any){
  const canonical=(value:any):string=>JSON.stringify(value,(_key,item)=>item&&typeof item==="object"&&!Array.isArray(item)?Object.fromEntries(Object.entries(item).sort(([a],[b])=>a.localeCompare(b))):item);
  await mountModelApplyHarness(fixture);
  const globals=window as any,harness=globals.__applyHarness,internals=globals.__TAURI_INTERNALS__,original=internals.invoke;
  const {useWorkspace}=await import("../src/stores/workspace");
  const projects=await original("list_projects",{});
  for(const project of projects){project.currentRevision=fixture.head;project.revisions[0].id=fixture.head;}
  useWorkspace.getState().setProject({...projects[0]});useWorkspace.getState().setSelected("body");
  let delayed=false,fail=false,finish:(()=>void)|null=null;
  internals.invoke=async(command:string,args:any)=>{
    if(command!=="measure_model_reference")return original(command,args);
    harness.calls.push({command,args});
    const report={...(fixture.reports[canonical(args.query)]??fixture.reports[args.query.kind]),projectId:args.projectId};
    if(canonical(report.query)!==canonical(args.query)||report.revisionId!==args.expectedRevision||report.bodyId!==args.bodyId)throw Error("Fixture query binding mismatch");
    if(delayed)await new Promise<void>(resolve=>{finish=resolve;});
    if(fail)throw JSON.stringify({code:"MEASUREMENT_FAILED",message:"isolated failure boundary"});
    return report;
  };
  Object.assign(harness,{
    setMeasurementDelayed:(value:boolean)=>{delayed=value;finish=null;},releaseMeasurement:()=>finish?.(),measurementPending:()=>!!finish,
    setMeasurementFailure:(value:boolean)=>{fail=value;},
    select:(kind:"body"|"edge"|"face")=>{
      const state=useWorkspace.getState();state.setSelected("body");
      if(kind==="edge")state.setSelectedEdge({bodyId:"body",edgeOrdinal:1,revisionId:fixture.head,topologyRef:fixture.edge});
      if(kind==="face")state.setSelectedFace({bodyId:"body",faceOrdinal:1,revisionId:fixture.head,topologyRef:fixture.face});
    },
    units:(units:"mm"|"cm"|"inch")=>{const state=useWorkspace.getState();state.setProject({...state.project!,units});},
  });
}
