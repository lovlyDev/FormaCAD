import type { Project } from "../../../../types";
import type { FaceSelection } from "../../faceSelection";
import { readTypedCadDocument } from "../../../../lib/typedCadDocument";
import { readTopologyReference } from "../../topology/topologyReference";
import { matchesReferenceRoute } from "../../topology/referenceRoute";
import type { FaceSectionReport } from "./faceSectionSchema";

export function captureFaceSection(project:Project|null,selection:FaceSelection|null,offsetMm:number,active:boolean){
 if(!active||!project?.currentRevision||selection?.revisionId!==project.currentRevision||!Number.isFinite(offsetMm)||Math.abs(offsetMm)>10000)return null;
 const revision=project.revisions.find(item=>item.id===project.currentRevision),rawProgram=revision?.program??"",document=readTypedCadDocument(rawProgram);
 const body=document?.bodies.find(item=>item.id===selection.bodyId),reference=readTopologyReference(selection.topologyRef);
 const source=project.files.find(file=>file.name===revision?.source&&/\.st(e)?p$/i.test(file.name));
 if(!document||document.revisionId!==project.currentRevision||!body||!reference||reference.kind!=="face"||reference.role==="cylinder-face:side"||!matchesReferenceRoute(document,body.sourceFeatureId,reference)||!source?.sha256||!source.size)return null;
 const request={projectId:project.id,expectedRevision:project.currentRevision,bodyId:body.id,query:{reference,offsetMm,deflectionMm:0.01}};
 return{key:JSON.stringify({request,rawProgram,sourceSha256:source.sha256,sourceSize:source.size}),request,rawProgram,sourceSha256:source.sha256,sourceSize:source.size};
}
export type FaceSectionCapture=NonNullable<ReturnType<typeof captureFaceSection>>;
export function matchesFaceSection(report:FaceSectionReport,capture:FaceSectionCapture){
 return report.projectId===capture.request.projectId&&report.revisionId===capture.request.expectedRevision&&report.bodyId===capture.request.bodyId&&report.sourceSha256===capture.sourceSha256&&report.sourceSize===capture.sourceSize&&JSON.stringify(report.query)===JSON.stringify(capture.request.query);
}
