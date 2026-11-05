import type { FolderSourceRef } from "../../features/project-sources/contract";
import { invoke } from "@tauri-apps/api/core";
import { native } from "../../lib/api";
import { readMeasurementReport } from "./measurementSchema";
import { reportMatchesCapture,type MeasurementCapture } from "./measurementCapture";

/** No renderer document, path, hash or measured value is sent to native IPC. */
export async function measureReference(captured:MeasurementCapture, source:FolderSourceRef|undefined=captured.source) {
  if(captured.sourceOpeningId&&(source?.sourceId!==captured.source?.sourceId||source?.commitId!==captured.source?.commitId||source?.taskScopeId!==captured.source?.taskScopeId))throw Error("MEASUREMENT_STALE");
  if(source&&!native)throw Error("PROJECT_FOLDER_UNSUPPORTED");
  if(!native)throw Error("MEASUREMENT_NATIVE_REQUIRED");
  const report=readMeasurementReport(await invoke("measure_model_reference",source ? {...captured.request, sourceId:source.sourceId, expectedCommitId:source.commitId} : captured.request));
  if(!reportMatchesCapture(report,captured))throw Error("MEASUREMENT_STALE");
  // Compare the raw saved source actually captured by this renderer. The host
  // independently checks committed bytes before queueing and before returning.
  const bytes=new TextEncoder().encode(captured.rawProgram);
  const hash=await crypto.subtle.digest("SHA-256",bytes);
  const documentSha256=Array.from(new Uint8Array(hash),byte=>byte.toString(16).padStart(2,"0")).join("");
  if(report.documentSha256!==documentSha256)throw Error("MEASUREMENT_STALE");
  return report;
}
