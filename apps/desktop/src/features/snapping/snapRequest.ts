import {useWorkspace} from "../../stores/workspace";
import {captureSceneRead,displayedSceneRead,type SceneRead} from "../project-sources/sceneRead";
import {useSnapSettings} from "./cursor/snapSettings";

export function captureSnapSource(){const state=useWorkspace.getState();return captureSceneRead(state.project,state.sourceBinding);}
export type SnapSource=SceneRead;
export function captureDisplayedSnapSource(object:object){
 const recorded=displayedSceneRead(object);if(recorded)return recorded;
 const fallback=captureSnapSource();return fallback?.source?null:fallback;
}

let latest:symbol|null=null;
let owner:{invalidate:()=>void}|null=null;

/** One latest cursor placement across pointer and pie-menu requests. Older native
 * work may finish, but cannot change another placement's pending/error/result. */
export function beginSnapRequest(source:SnapSource|null,displayed:()=>boolean=()=>true){
 owner?.invalidate();
 const token=Symbol("snap placement");latest=token;
 let revoked=false,finished=false,failed=false;
 let unsubscribe=()=>{};
 const isCurrent=()=>!revoked&&latest===token&&displayed()&&(source?source.isCurrent():!useWorkspace.getState().project);
 const controller={invalidate:()=>{
  if(revoked)return;revoked=true;unsubscribe();
  if(latest===token){useSnapSettings.getState().setPending(false);useSnapSettings.getState().setError(null);}
  if(owner===controller)owner=null;
 }};
 owner=controller;
 useSnapSettings.getState().setError(null);
 unsubscribe=useWorkspace.subscribe(()=>{if(!isCurrent())controller.invalidate();});
 return{
  isCurrent,
  pending(){if(!finished&&isCurrent())useSnapSettings.getState().setPending(true);},
  fail(cause:unknown){if(!finished&&isCurrent()){failed=true;useSnapSettings.getState().setError(cause);}},
  finish(){
   if(finished)return;finished=true;
   if(latest===token)useSnapSettings.getState().setPending(false);
   // A displayed failure retains one bounded watcher until source change or
   // next intent; it must not survive into a different project/opening.
   if(!failed||!isCurrent()){unsubscribe();if(owner===controller)owner=null;}
  },
 };
}
