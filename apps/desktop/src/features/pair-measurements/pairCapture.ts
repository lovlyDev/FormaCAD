import type { Project } from "../../types";
import type { EdgeSelection } from "../viewer/edgeSelection";
import type { FaceSelection } from "../viewer/faceSelection";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import { readTopologyReference,type TopologyReference } from "../viewer/topology/topologyReference";
import { matchesReferenceRoute } from "../viewer/topology/referenceRoute";
import { pairQuerySchema,type PairKind,type PairReport } from "./pairSchema";

export function capturePairScope(project:Project|null,bodyId:string|null,sceneToken:string,interactive:boolean){
 if(!interactive||!project?.currentRevision||!bodyId||!sceneToken)return null;
 const revision=project.revisions.find(value=>value.id===project.currentRevision),rawProgram=revision?.program??"",document=readTypedCadDocument(rawProgram);
 const body=document?.bodies.find(value=>value.id===bodyId),source=project.files.find(value=>value.name===revision?.source&&/\.st(e)?p$/i.test(value.name));
 if(!document||document.revisionId!==project.currentRevision||!body||!source?.sha256||!/^[a-f0-9]{64}$/.test(source.sha256)||typeof source.size!=="number"||!Number.isInteger(source.size)||source.size<=0||source.size>40*1024*1024)return null;
 const binding={projectId:project.id,expectedRevision:project.currentRevision,bodyId,rawProgram,sourceSha256:source.sha256,sourceSize:source.size,sceneToken};
 return {...binding,key:JSON.stringify(binding),document,body};
}
export type PairScope=NonNullable<ReturnType<typeof capturePairScope>>;
/** Copies ONLY the real current selection's validated authored identity. */
export function currentPairReference(scope:PairScope|null,edge:EdgeSelection|null,face:FaceSelection|null):TopologyReference|null {
 if(!scope|| (!!edge&&!!face))return null;
 const selected=edge??face;
 if(!selected||selected.bodyId!==scope.bodyId||selected.revisionId!==scope.expectedRevision||selected.sceneToken!==scope.sceneToken)return null;
 const reference=readTopologyReference(selected.topologyRef);
 if(!reference||(edge&&reference.kind!=="edge")||(face&&reference.kind!=="face")||!matchesReferenceRoute(scope.document,scope.body.sourceFeatureId,reference))return null;
 return reference;
}
export function pairKinds(first:TopologyReference|null,second:TopologyReference|null):PairKind[]{
 if(!first||!second)return[];
 const kinds:PairKind[]=["minimumDistance"];
 if(first.kind==="face"&&second.kind==="face"&&first.role!=="cylinder-face:side"&&second.role!=="cylinder-face:side")kinds.push("faceNormalAngle");
 if(first.kind==="edge"&&second.kind==="edge"&&first.role.startsWith("box-edge:")&&second.role.startsWith("box-edge:"))kinds.push("edgeAcuteAngle");
 return kinds;
}
export function capturePair(scope:PairScope|null,first:TopologyReference|null,second:TopologyReference|null,kind:PairKind){
 if(!scope||!first||!second||!pairKinds(first,second).includes(kind)||![first,second].every(reference=>matchesReferenceRoute(scope.document,scope.body.sourceFeatureId,reference)))return null;
 const parsed=pairQuerySchema.safeParse({kind,first,second});if(!parsed.success)return null;
 const request={projectId:scope.projectId,expectedRevision:scope.expectedRevision,bodyId:scope.bodyId,query:parsed.data};
 return{key:JSON.stringify([scope.key,parsed.data]),request,rawProgram:scope.rawProgram,sourceSha256:scope.sourceSha256,sourceSize:scope.sourceSize};
}
export type PairCapture=NonNullable<ReturnType<typeof capturePair>>;
export function matchesPairReport(report:PairReport,capture:PairCapture){
 const expected=pairQuerySchema.safeParse(capture.request.query);
 return expected.success&&report.projectId===capture.request.projectId&&report.revisionId===capture.request.expectedRevision&&report.bodyId===capture.request.bodyId&&report.sourceSha256===capture.sourceSha256&&report.sourceSize===capture.sourceSize&&JSON.stringify(report.query)===JSON.stringify(expected.data);
}
