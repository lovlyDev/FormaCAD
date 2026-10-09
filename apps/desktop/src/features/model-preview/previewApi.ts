import { invoke,isTauri } from "@tauri-apps/api/core";
import type { Group } from "three";
import { loadModel } from "../../lib/files";
import { decodePreviewPacket,type PreviewMetrics } from "./previewPacket";
export type ModelPreview={object:Group;metrics:PreviewMetrics};
export const modelPreviewAvailable=()=>isTauri();
export async function previewModel(projectId:string,source:string,expectedRevision:string|null,shouldContinue:()=>boolean=()=>true):Promise<ModelPreview>{
 const hash=Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256",new TextEncoder().encode(source)))).map(byte=>byte.toString(16).padStart(2,"0")).join("");
 if(!shouldContinue())throw new Error("Model preview cancelled");
 const response=await invoke<ArrayBuffer|number[]>("preview_model",{projectId,program:source,expectedRevision});
 const packet=response instanceof ArrayBuffer?response:new Uint8Array(response).buffer;
 const {metrics,glb}=decodePreviewPacket(packet,hash),url=URL.createObjectURL(new Blob([glb],{type:"model/gltf-binary"}));
 try{return {metrics,object:await loadModel({name:"preview.glb",size:glb.byteLength,kind:"model",data:url})};}finally{URL.revokeObjectURL(url);}
}
export async function cancelModelPreview(projectId:string):Promise<void>{await invoke("cancel_model_preview",{projectId});}
