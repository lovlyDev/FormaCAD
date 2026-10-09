import type { Project } from "../../types";
import type { EdgeSelection } from "../viewer/edgeSelection";
import type { FaceSelection } from "../viewer/faceSelection";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import { matchesReferenceRoute } from "../viewer/topology/referenceRoute";
import { readTopologyReference } from "../viewer/topology/topologyReference";
import type { MeasurementQuery } from "./measurementSchema";

/** Capability checks never synthesize references from ordinals or mesh shape. */
export function supportedMeasurementQueries(project:Project|null,bodyId:string|null,edge:EdgeSelection|null,face:FaceSelection|null):MeasurementQuery[]{
  const revision=project?.revisions.find(item=>item.id===project.currentRevision),document=readTypedCadDocument(revision?.program??"");
  const body=document?.bodies.find(item=>item.id===bodyId);
  if(!document||!body)return[];
  const queries:MeasurementQuery[]=[{kind:"bodyMetrics"}];
  const selected=edge??face;
  if(!selected||selected.bodyId!==bodyId||selected.revisionId!==project?.currentRevision)return queries;
  const reference=readTopologyReference(selected.topologyRef);
  if(!reference||!matchesReferenceRoute(document,body.sourceFeatureId,reference))return queries;
  if(edge&&reference.kind==="edge"){
    queries.push({kind:"edgeLength",reference});
    if(reference.role.startsWith("cylinder-edge:"))queries.push({kind:"edgeRadius",reference},{kind:"edgeDiameter",reference});
  }
  if(face&&reference.kind==="face"){
    queries.push({kind:"faceArea",reference});
    if(reference.role!=="cylinder-face:side")queries.push({kind:"planarFace",reference});
  }
  return queries;
}
