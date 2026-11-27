import {z} from "zod";
import type {FolderSourceRef} from "./contract";
import type {ReviewPlan} from "../agents/review/plan/reviewPlan";

export type FolderWriteIntent=
 | Readonly<{kind:"model";program:string;prompt:string;expectedRevision:string|null;reviewPlan?:ReviewPlan|null}>
 | Readonly<{kind:"commands";commands:readonly object[];prompt:string;expectedRevision:string}>
 | Readonly<{kind:"history";direction:"undo"|"redo";expectedRevision:string|null}>;
export interface FolderPermissionOptions { readonly source:FolderSourceRef; readonly intent:FolderWriteIntent }
export interface FolderPermissionContext { readonly projectId:string; readonly source:FolderSourceRef }
export interface FolderPermissionTicket extends FolderPermissionContext { readonly id:string; readonly requiresConfirmation:boolean }

const receipt=z.object({id:z.string().uuid(),requiresConfirmation:z.boolean()});
const changed=()=>new Error("PROJECT_SNAPSHOT_CHANGED");
export function capturePermissionContext(value:FolderPermissionContext):Readonly<FolderPermissionContext>{
 if(!value?.projectId||!value.source?.sourceId||!value.source.commitId||!value.source.taskScopeId)throw changed();
 return Object.freeze({projectId:value.projectId,source:Object.freeze({sourceId:value.source.sourceId,commitId:value.source.commitId,taskScopeId:value.source.taskScopeId})});
}
export function capturePermissionRequest(projectId:string,options:FolderPermissionOptions){
 if(!options?.intent)throw changed();
 const context=capturePermissionContext({projectId,source:options.source});
 return{context,intent:Object.freeze(structuredClone(options.intent))};
}
export function readPermissionTicket(value:unknown,context:Readonly<FolderPermissionContext>):Readonly<FolderPermissionTicket>{
 const parsed=receipt.safeParse(value);if(!parsed.success)throw changed();
 return Object.freeze({...context,...parsed.data});
}
