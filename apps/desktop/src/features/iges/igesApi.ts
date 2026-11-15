import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { getLocale } from "../../i18n";
import { native } from "../../lib/api";
import type { Project } from "../../types";
import { igesReportSchema } from "./igesSchema";
export interface IgesInput { projectId: string; expectedRevision: string | null; name: string; data: string }
export function isIgesName(name: string) { return /\.(igs|iges)$/i.test(name); }
export function validIgesFile(file: Pick<File, "name" | "size">) {
  return isIgesName(file.name) && file.name.length > 0 && [...file.name].length <= 120 && file.size > 0 && file.size <= 40 * 1024 * 1024;
}
export async function readIgesInput(project: Project, file: File): Promise<IgesInput> {
  if (!native) throw Error("IGES_NATIVE_REQUIRED");
  if (!validIgesFile(file)) throw Error("IGES_INVALID_INPUT");
  const data = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader(); reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(Error("IGES_INVALID_INPUT")); reader.readAsDataURL(file);
  });
  return { projectId: project.id, expectedRevision: project.currentRevision, name: file.name, data };
}
const previewSchema = z.strictObject({ report: igesReportSchema, previewData: z.string().startsWith("data:model/gltf-binary;base64,").max(40 * 1024 * 1024 * 4 / 3 + 1024) });
async function dataSeal(data:string){
  try{
    const bytes=await(await fetch(data)).arrayBuffer();
    const hash=Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256",bytes)),b=>b.toString(16).padStart(2,"0")).join("");
    return {size:bytes.byteLength,hash};
  }catch{throw Error("IGES_REPORT_INVALID");}
}
export async function previewIges(input: IgesInput) {
  const [source,raw]=await Promise.all([dataSeal(input.data),invoke("preview_iges_import",{request:input})]);
  const parsed=previewSchema.safeParse(raw);
  if(!parsed.success)throw Error("IGES_REPORT_INVALID");
  const result=parsed.data;
  if (result.report.direction !== "import") throw Error("IGES_REPORT_INVALID");
  const preview=await dataSeal(result.previewData);
  if(source.hash!==result.report.sourceSha256 || source.size!==result.report.sourceSize || preview.size!==result.report.previewSize || preview.hash!==result.report.previewSha256)throw Error("IGES_REPORT_INVALID");
  return result;
}
export async function commitIges(input: IgesInput, sourceSha256: string, prompt: string) {
  const result = await invoke<{ project: Project; report: unknown }>("import_iges", { request: { input, sourceSha256, prompt } });
  const parsed=igesReportSchema.safeParse(result?.report);
  if(!parsed.success)throw Error("IGES_REPORT_INVALID");
  const report=parsed.data;
  if (report.direction !== "import" || report.sourceSha256 !== sourceSha256 || result.project?.id !== input.projectId) throw Error("IGES_REPORT_INVALID");
  return result.project;
}
const receiptSchema = z.strictObject({ path: z.string().min(1), revisionId: z.string(), bodyId: z.string().nullable(), report: igesReportSchema });
export type IgesExportReceipt = z.infer<typeof receiptSchema>;
export async function exportIges(project: Project, bodyId: string | null): Promise<IgesExportReceipt | null> {
  if (!native) throw Error("IGES_NATIVE_REQUIRED");
  if (!project.currentRevision) throw Error("IGES_EXACT_SOURCE_REQUIRED");
  const result = await invoke("export_iges", { request: { projectId: project.id, expectedRevision: project.currentRevision, bodyId }, locale: getLocale() });
  if (result === null) return null;
  const parsed=receiptSchema.safeParse(result);
  if(!parsed.success)throw Error("IGES_REPORT_INVALID");
  const receipt=parsed.data;
  if (receipt.revisionId !== project.currentRevision || receipt.bodyId !== bodyId || receipt.report.direction !== "export") throw Error("IGES_REPORT_INVALID");
  return receipt;
}
