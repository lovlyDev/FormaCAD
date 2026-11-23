import {useWorkspace} from "../../stores/workspace";
import type {Project} from "../../types";
import {isSourceCurrent,sourceRefFor,type ProjectSourceBinding} from "./sourceBinding";

function geometry(project:Project){
 const revision=project.revisions.find(item=>item.id===project.currentRevision);
 const file=project.files.find(item=>item.name===revision?.source);
 return[project.currentRevision,revision?.program,revision?.source,revision?.programBase,file?.sha256,file?.size] as const;
}
function bind(projectId:string,revisionId:Project["currentRevision"],seal:ReturnType<typeof geometry>,binding:ProjectSourceBinding){
 const source=sourceRefFor(binding,projectId);
 const isOpeningCurrent=()=>{
  const state=useWorkspace.getState();
  return !!state.project&&state.project.id===projectId&&isSourceCurrent(state.sourceBinding,binding);
 };
 return{projectId,revisionId,source,isOpeningCurrent,isCurrent:()=>{
  const state=useWorkspace.getState();
  if(!state.project||!isOpeningCurrent())return false;
  const current=geometry(state.project);return seal.every((value,index)=>value===current[index]);
 }};
}
/** Primitive original fields only; the closure does not retain a full Project. */
export function captureSceneRead(project:Project|null,binding?:ProjectSourceBinding|null){
 if(!project||!binding||binding.projectId!==project.id)return null;
 return bind(project.id,project.currentRevision,geometry(project),binding);
}
export type SceneRead=NonNullable<ReturnType<typeof captureSceneRead>>;

// Never use userData or serialized extras for process-local source references.
const displayed=new WeakMap<object,SceneRead>();
export function rememberSceneRead(object:object,source:SceneRead|null){if(source)displayed.set(object,source);}
export function displayedSceneRead(object:object){return displayed.get(object);}
