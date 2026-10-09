import { mountCircularMeasurementHarness } from "./circularMeasurementHarness";
export async function mountFaceSectionHarness(fixture:any){
 await mountCircularMeasurementHarness(fixture);const globals=window as any,internals=globals.__TAURI_INTERNALS__,original=internals.invoke,harness=globals.__applyHarness;
 const canonical=(value:any)=>JSON.stringify(value,(_key,item)=>item&&typeof item==="object"&&!Array.isArray(item)?Object.fromEntries(Object.entries(item).sort(([a],[b])=>a.localeCompare(b))):item),{useWorkspace}=await import("../src/stores/workspace"),Fiber=await import("@react-three/fiber");
 let delayed=false,finish:(()=>void)|null=null,fail=false;
 internals.invoke=async(command:string,args:any)=>{
  if(command==="section_model")return fixture.manualReport;
  if(command!=="section_model_reference")return original(command,args);
  harness.calls.push({command,args});const found=fixture.faceReports[canonical(args.query)];if(!found)throw Error("Unexpected native fixture section query");const report={...found,projectId:args.projectId};if(report.revisionId!==args.expectedRevision||report.bodyId!==args.bodyId)throw Error("Face-section binding mismatch");
  if(delayed)await new Promise<void>(resolve=>{finish=resolve;});if(fail)throw JSON.stringify({code:"SECTION_STALE",message:"isolated seal changed"});return report;
 };
 Object.assign(harness,{delaySection:(value:boolean)=>{delayed=value;finish=null;},sectionPending:()=>!!finish,releaseSection:()=>finish?.(),failSection:(value:boolean)=>{fail=value;},
  sectionInspection:()=>{const canvas=document.querySelector(".viewer-area canvas") as HTMLCanvasElement,live=Fiber._roots.get(canvas)?.store.getState(),body=live?.scene.getObjectByName("body") as any;const plane=body?.material.clippingPlanes?.[0];return{clipping:body?.material.clippingPlanes?.length??0,plane:plane?{normal:plane.normal.toArray(),constant:plane.constant}:null,camera:[...live!.camera.position.toArray(),...live!.camera.quaternion.toArray()]};},
  sourceSeal:(sha256:string)=>{const state=useWorkspace.getState();state.setProject({...state.project!,files:state.project!.files.map(file=>file.name==="base.step"?{...file,sha256}:file)});},
 });
}
