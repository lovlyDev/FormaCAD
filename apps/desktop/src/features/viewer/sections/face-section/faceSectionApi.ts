import type { FolderSourceRef } from "../../../project-sources/contract";
import { invoke } from "@tauri-apps/api/core";
import { native } from "../../../../lib/api";
import { matchesFaceSection,type FaceSectionCapture } from "./faceSectionCapture";
import { readFaceSectionReport } from "./faceSectionSchema";
export async function sectionReference(captured:FaceSectionCapture, source?:FolderSourceRef){
 if(source&&!native)throw Error("PROJECT_FOLDER_UNSUPPORTED");
  if(!native)throw Error("SECTION_NATIVE_REQUIRED");
 const report=readFaceSectionReport(await invoke("section_model_reference",source ? {...captured.request, sourceId:source.sourceId, expectedCommitId:source.commitId} : captured.request));
 if(!matchesFaceSection(report,captured))throw Error("SECTION_STALE");
 const hash=await crypto.subtle.digest("SHA-256",new TextEncoder().encode(captured.rawProgram));
 if(report.documentSha256!==Array.from(new Uint8Array(hash),byte=>byte.toString(16).padStart(2,"0")).join(""))throw Error("SECTION_STALE");
 return report;
}
