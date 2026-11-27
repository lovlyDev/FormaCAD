import type { FolderSourceRef } from "../project-sources/contract";
import { invoke } from "@tauri-apps/api/core";
import { native } from "../../lib/api";
import { matchesPairReport,type PairCapture } from "./pairCapture";
import { readPairReport } from "./pairSchema";
export async function measurePair(capture:PairCapture, source:FolderSourceRef|undefined=capture.source){
 if(capture.sourceOpeningId&&(source?.sourceId!==capture.source?.sourceId||source?.commitId!==capture.source?.commitId||source?.taskScopeId!==capture.source?.taskScopeId))throw Error("MEASUREMENT_STALE");
 if(source&&!native)throw Error("PROJECT_FOLDER_UNSUPPORTED");
  if(!native)throw Error("MEASUREMENT_UNSUPPORTED");
 const report=readPairReport(await invoke("measure_model_pair",source ? {...capture.request, sourceId:source.sourceId, expectedCommitId:source.commitId} : capture.request));
 if(!matchesPairReport(report,capture))throw Error("MEASUREMENT_STALE");
 const hash=await crypto.subtle.digest("SHA-256",new TextEncoder().encode(capture.rawProgram));
 if(report.documentSha256!==Array.from(new Uint8Array(hash),value=>value.toString(16).padStart(2,"0")).join(""))throw Error("MEASUREMENT_STALE");
 return report;
}
