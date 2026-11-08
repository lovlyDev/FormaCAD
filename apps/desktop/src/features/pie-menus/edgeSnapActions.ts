import {useWorkspace} from "../../stores/workspace";
import {useTransformViewport} from "../transform/viewport/transformStore";
import {exactSnap} from "../snapping/snapApi";
import {beginSnapRequest,captureSnapSource,captureDisplayedSnapSource} from "../snapping/snapRequest";

/** A pie opened on one selection never retargets a newer selection/source. */
export function edgeSnapActions(object?:{uuid:string}){
 const source=captureSnapSource(),edge=useWorkspace.getState().selectedEdge;
 const displayed=object?captureDisplayedSnapSource(object):source;
 const displayedCurrent=()=>!object||!!displayed?.isCurrent();
 const ready=displayedCurrent()&&(!object||!edge?.sceneToken||edge.sceneToken===object.uuid)&&!!source?.revisionId&&edge?.revisionId===source.revisionId&&!!edge?.topologyRef;
 return{ready,run(kind:"edgeMidpoint"|"circleCenter"){
  if(!ready||!source?.revisionId||!edge?.topologyRef||!source.isCurrent()||!displayedCurrent()||useWorkspace.getState().selectedEdge!==edge)return;
  const request=beginSnapRequest(source,()=>displayedCurrent()&&useWorkspace.getState().selectedEdge===edge);
  if(!request.isCurrent()){request.finish();return;}
  request.pending();
  const scope={projectId:source.projectId,revisionId:source.revisionId,bodyId:edge.bodyId};
  const query={kind,reference:edge.topologyRef};
  return(source.source?exactSnap(scope,query,source.source):exactSnap(scope,query)).then(result=>{
   if(request.isCurrent())useTransformViewport.getState().setCursor(result.pointMm);
  },cause=>request.fail(cause)).finally(()=>request.finish());
 }};
}
