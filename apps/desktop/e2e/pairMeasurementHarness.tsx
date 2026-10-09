import { mountCircularMeasurementHarness } from "./circularMeasurementHarness";
export async function mountPairMeasurementHarness(fixture:any){
 await mountCircularMeasurementHarness(fixture);
 const globals=window as any,harness=globals.__applyHarness,internals=globals.__TAURI_INTERNALS__,original=internals.invoke;
 const canonical=(value:any):string=>JSON.stringify(value,(_key,item)=>item&&typeof item==="object"&&!Array.isArray(item)?Object.fromEntries(Object.entries(item).sort(([a],[b])=>a.localeCompare(b))):item);
 const {useWorkspace}=await import("../src/stores/workspace");let delayed=false,finish:(()=>void)|null=null;
 internals.invoke=async(command:string,args:any)=>{
  if(command!=="measure_model_pair")return original(command,args);harness.calls.push({command,args});
  const report=fixture.pairReports[canonical(args.query)];if(!report||report.revisionId!==args.expectedRevision||report.bodyId!==args.bodyId)throw Error("Fixture pair binding mismatch");
  if(delayed)await new Promise<void>(resolve=>{finish=resolve;});return{...report,projectId:args.projectId};
 };
 Object.assign(harness,{delayPair:(value:boolean)=>{delayed=value;finish=null;},pairPending:()=>!!finish,releasePair:()=>finish?.(),sourceSeal:(sha256:string)=>{const state=useWorkspace.getState();state.setProject({...state.project!,files:state.project!.files.map(file=>file.name==="base.step"?{...file,sha256}:file)});},pairQueries:Object.values(fixture.pairReports).map((report:any)=>report.query)});
}
