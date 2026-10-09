import type { Project } from "../../types";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import { measurementQuerySchema,type MeasurementQuery,type MeasurementReport } from "./measurementSchema";

export interface MeasurementContext {project:Project|null;bodyId:string|null;query:MeasurementQuery;selectionToken:string;interactive:boolean}
export interface MeasurementCapture {
  key:string;request:{projectId:string;expectedRevision:string;bodyId:string;query:MeasurementQuery};
  sourceSha256:string;sourceSize:number;rawProgram:string;selectionToken:string;
}
export function captureMeasurement(context:MeasurementContext):MeasurementCapture|null {
  const {project,bodyId,selectionToken,interactive}=context;
  if(!project?.currentRevision||!bodyId||!interactive)return null;
  const revision=project.revisions.find(item=>item.id===project.currentRevision),document=readTypedCadDocument(revision?.program??"");
  const source=project.files.find(file=>file.name===revision?.source&&/\.st(e)?p$/i.test(file.name));
  const query=measurementQuerySchema.safeParse(context.query);
  if(!document||document.revisionId!==revision?.id||!document.bodies.some(body=>body.id===bodyId)||!query.success||!source?.sha256||!source.size)return null;
  const request={projectId:project.id,expectedRevision:project.currentRevision,bodyId,query:query.data};
  const rawProgram=revision!.program!;
  const key=JSON.stringify({request,sourceSha256:source.sha256,sourceSize:source.size,rawProgram,selectionToken});
  return Object.freeze({key,request,sourceSha256:source.sha256,sourceSize:source.size,rawProgram,selectionToken});
}
export function reportMatchesCapture(report:MeasurementReport,captured:MeasurementCapture):boolean {
  return report.projectId===captured.request.projectId&&report.revisionId===captured.request.expectedRevision&&report.bodyId===captured.request.bodyId
    &&report.sourceSha256===captured.sourceSha256&&report.sourceSize===captured.sourceSize
    &&JSON.stringify(report.query)===JSON.stringify(captured.request.query);
}
