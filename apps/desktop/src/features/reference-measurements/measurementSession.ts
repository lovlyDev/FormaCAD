import type { MeasurementCapture } from "./measurementCapture";
import { reportMatchesCapture } from "./measurementCapture";
import type { MeasurementReport } from "./measurementSchema";
export interface MeasurementState {key:string;phase:"idle"|"pending"|"ready"|"failed";working:boolean;report:MeasurementReport|null;error?:unknown}

/** Local cancellation stops adoption only; it does not claim to cancel a native worker. */
export class MeasurementSession {
  private epoch=0;private context:MeasurementCapture|null=null;private inFlight=false;
  private value:MeasurementState={key:"",phase:"idle",working:false,report:null};private listeners=new Set<()=>void>();
  subscribe=(listener:()=>void)=>{this.listeners.add(listener);return()=>{this.listeners.delete(listener);};};
  snapshot=()=>this.value;
  private publish(value:MeasurementState){this.value=value;this.listeners.forEach(listener=>listener());}
  setContext(context:MeasurementCapture|null){
    if(context?.key===this.context?.key)return;
    this.epoch++;this.context=context;this.publish({key:context?.key??"",phase:"idle",working:this.inFlight,report:null});
  }
  stopWaiting(){this.epoch++;this.publish({...this.value,phase:this.value.report?"ready":"idle",error:undefined});}
  async run(load:(capture:MeasurementCapture)=>Promise<MeasurementReport>){
    const captured=this.context;if(!captured||this.inFlight)return;
    const epoch=++this.epoch;this.inFlight=true;this.publish({...this.value,phase:"pending",working:true,error:undefined});
    try {
      const report=await load(captured);
      if(epoch!==this.epoch||captured.key!==this.context?.key)return;
      if(!reportMatchesCapture(report,captured))throw Error("MEASUREMENT_STALE");
      this.publish({key:captured.key,phase:"ready",working:true,report});
    }catch(error){if(epoch===this.epoch&&captured.key===this.context?.key)this.publish({...this.value,phase:"failed",error});}
    finally{this.inFlight=false;this.publish({...this.value,working:false});}
  }
  close(){this.epoch++;this.context=null;this.listeners.clear();}
}
