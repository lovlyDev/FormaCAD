import type { Vec3 } from "../transform/frames";
import { exactSnap } from "./snapApi";
import type { SnapCandidate } from "./snapCandidate";
import { useSnapSettings } from "./cursor/snapSettings";
import {beginSnapRequest,captureSnapSource} from "./snapRequest";

export interface ResolvedSnap {
 pointMm:Vec3;
 exact:boolean;
 /** Recheck again at the caller's adoption boundary after Promise settlement. */
 isCurrent:()=>boolean;
}

/** Native refinement always uses the original source; approximation stays local. */
export async function resolveSnap(candidate:SnapCandidate,displayed:()=>boolean=()=>true):Promise<ResolvedSnap|null>{
 const source=captureSnapSource(),request=beginSnapRequest(source,displayed);
 try{
  if(!request.isCurrent())return null;
  if(!useSnapSettings.getState().exact||!candidate.query||!source?.revisionId)
   return{pointMm:candidate.pointMm,exact:false,isCurrent:request.isCurrent};
  request.pending();
  const scope={projectId:source.projectId,revisionId:source.revisionId,bodyId:candidate.bodyId};
  const result=await(source.source?exactSnap(scope,candidate.query,source.source):exactSnap(scope,candidate.query));
  if(!request.isCurrent())return null;
  return{pointMm:result.pointMm,exact:true,isCurrent:request.isCurrent};
 }catch(cause){
  if(!request.isCurrent())return null;
  request.fail(cause);
  if(/PROJECT_SNAPSHOT_CHANGED|MEASUREMENT_STALE|SNAP_REPORT_INVALID/.test(String(cause)))return null;
  return{pointMm:candidate.pointMm,exact:false,isCurrent:request.isCurrent};
 }finally{request.finish();}
}
