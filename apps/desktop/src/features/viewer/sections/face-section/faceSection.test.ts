import { expect,it } from "vitest";
import { newProject } from "../../../../stores/workspace";
import { defaults } from "../../../../types";
import { captureFaceSection,matchesFaceSection } from "./faceSectionCapture";
import { faceSectionQuerySchema,faceSectionReportSchema } from "./faceSectionSchema";
const head="c661482b-378a-4fa3-a426-ad9bf6621478",sha="a".repeat(64);
const reference={schemaVersion:1 as const,kind:"face" as const,ownerFeatureId:"pad",role:"cylinder-face:top" as const,occurrencePath:[]};
function context(){
 const project=newProject("Section","blank","mm","codex");project.currentRevision=head;
 const program=JSON.stringify({schemaVersion:2,revisionId:head,parameters:[],features:[{id:"profile",name:"Profile",operation:{type:"circle"}},{id:"pad",name:"Pad",operation:{type:"extrude",sketchId:"profile"}}],bodies:[{id:"body",name:"Body",sourceFeatureId:"pad"}]});
 project.revisions=[{id:head,parent:null,createdAt:project.createdAt,prompt:"Source",parameters:defaults,program,source:"source.step"}];project.files=[{name:"source.step",kind:"model",sha256:sha,size:100}];
 return{project,selection:{bodyId:"body",revisionId:head,faceOrdinal:99,topologyRef:reference}};
}
it("captures authored planar identity while refusing curved, ordinal-only or stale geometry",()=>{
 const{project,selection}=context(),captured=captureFaceSection(project,selection,-2,true)!;
 expect(captured.request.query).toEqual({reference,offsetMm:-2,deflectionMm:.01});expect(captured.request).not.toHaveProperty("originMm");
 expect(captureFaceSection(project,{...selection,topologyRef:undefined},-2,true)).toBeNull();
 expect(captureFaceSection(project,{...selection,topologyRef:{...reference,role:"cylinder-face:side"}},-2,true)).toBeNull();
 expect(captureFaceSection(project,{...selection,revisionId:"older"},-2,true)).toBeNull();expect(captureFaceSection(project,selection,-2,false)).toBeNull();
 expect(captureFaceSection(project,selection,10001,true)).toBeNull();
});
it("strictly rejects renderer plane authority, signed-normal inversion and offset mismatch",()=>{
 const{project,selection}=context(),capture=captureFaceSection(project,selection,-2,true)!;
 expect(faceSectionQuerySchema.safeParse({...capture.request.query,normal:[0,0,1]}).success).toBe(false);
 const report={projectId:project.id,schemaVersion:1,requestId:"0fdca3bb-60ac-4014-99e9-28b8306687c7",engine:{name:"occt",protocolVersion:1},evaluationSource:"rebuiltAuthoredFaceAndSealedStep",revisionId:head,bodyId:"body",documentSha256:sha,sourceSha256:sha,sourceSize:100,importedAssetSeals:[],query:capture.request.query,face:{areaMm2:20,originMm:[0,0,10],normal:[0,0,1]},geometry:{plane:{originMm:[0,0,8],normal:[0,0,1],deflectionMm:.01},curves:[],totalLengthMm:0}};
 const parsed=faceSectionReportSchema.parse(report);expect(matchesFaceSection(parsed,capture)).toBe(true);
 expect(faceSectionReportSchema.safeParse({...report,geometry:{...report.geometry,plane:{...report.geometry.plane,normal:[0,0,-1]}}}).success).toBe(false);
 expect(faceSectionReportSchema.safeParse({...report,geometry:{...report.geometry,plane:{...report.geometry.plane,originMm:[0,0,12]}}}).success).toBe(false);
 expect(matchesFaceSection({...parsed,sourceSha256:"b".repeat(64)},capture)).toBe(false);
});
