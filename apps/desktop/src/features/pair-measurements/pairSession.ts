import { matchesPairReport,type PairCapture } from "./pairCapture";
import type { PairReport } from "./pairSchema";
export interface PairState {key:string;phase:"idle"|"pending"|"ready"|"failed";working:boolean;report:PairReport|null;error?:unknown}
/** Stop waiting invalidates adoption; the backend task continues until actually finished/cancelled. */
export class PairSession {
 private epoch=0;private context:PairCapture|null=null;private inFlight=false;private closed=false;
 private value:PairState={key:"",phase:"idle",working:false,report:null};private listeners=new Set<()=>void>();
 subscribe=(listener:()=>void)=>{this.listeners.add(listener);return()=>{this.listeners.delete(listener);};};
 snapshot=()=>this.value;
 private publish(value:PairState){this.value=value;this.listeners.forEach(listener=>listener());}
 setContext(context:PairCapture|null){if(this.closed||context?.key===this.context?.key)return;this.epoch++;this.context=context;this.publish({key:context?.key??"",phase:"idle",working:this.inFlight,report:null});}
 stopWaiting(){if(this.closed)return;this.epoch++;this.publish({...this.value,phase:this.value.report?"ready":"idle",error:undefined});}
 async run(load:(capture:PairCapture)=>Promise<PairReport>){
  const capture=this.context;if(this.closed||!capture||this.inFlight)return;
  const epoch=++this.epoch;this.inFlight=true;this.publish({...this.value,phase:"pending",working:true,error:undefined});
  try{const report=await load(capture);if(this.closed||epoch!==this.epoch||capture.key!==this.context?.key)return;if(!matchesPairReport(report,capture))throw Error("MEASUREMENT_STALE");this.publish({key:capture.key,phase:"ready",working:true,report});}
  catch(error){if(!this.closed&&epoch===this.epoch&&capture.key===this.context?.key)this.publish({...this.value,phase:"failed",error});}
  finally{this.inFlight=false;if(!this.closed)this.publish({...this.value,working:false});}
 }
 close(){this.epoch++;this.closed=true;this.context=null;this.listeners.clear();}
}
