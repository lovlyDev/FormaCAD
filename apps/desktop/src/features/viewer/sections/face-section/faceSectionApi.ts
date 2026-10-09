import { invoke } from "@tauri-apps/api/core";
import { native } from "../../../../lib/api";
import { matchesFaceSection,type FaceSectionCapture } from "./faceSectionCapture";
import { readFaceSectionReport } from "./faceSectionSchema";
export async function sectionReference(captured:FaceSectionCapture){
 if(!native)throw Error("SECTION_NATIVE_REQUIRED");
 const report=readFaceSectionReport(await invoke("section_model_reference",captured.request));
 if(!matchesFaceSection(report,captured))throw Error("SECTION_STALE");
 const hash=await crypto.subtle.digest("SHA-256",new TextEncoder().encode(captured.rawProgram));
 if(report.documentSha256!==Array.from(new Uint8Array(hash),byte=>byte.toString(16).padStart(2,"0")).join(""))throw Error("SECTION_STALE");
 return report;
}
