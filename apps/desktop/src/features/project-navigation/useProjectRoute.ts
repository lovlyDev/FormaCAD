import {useEffect,useRef} from "react";
import {useNavigate,useParams} from "react-router-dom";
import {useQueryClient} from "@tanstack/react-query";
import {listProjects} from "../../lib/api";
import {errorText} from "../../i18n";
import {useWorkspace} from "../../stores/workspace";
import type {Project} from "../../types";

/** A cached list is not authoritative evidence that a direct project link is missing. */
export function useProjectRoute(projects:Project[]|undefined){
 const {projectId}=useParams(),nav=useNavigate(),query=useQueryClient();
 const setProject=useWorkspace(state=>state.setProject),setError=useWorkspace(state=>state.setError);
 const current=useRef(projectId);current.current=projectId;
 useEffect(()=>{
  const activeProject=useWorkspace.getState().project;
  if(!projectId){if(activeProject)setProject(null);return;}
  if(!projects||activeProject?.id===projectId)return;
  const cached=projects.find(project=>project.id===projectId);
  if(cached){setProject(cached);return;}
  // Never leave another project's editable scene under an unresolved new URL.
  if(activeProject)setProject(null);
  let active=true;
  // One fresh read on a cache miss. Read errors stay visible; no failed IPC retry.
  void listProjects().then(fresh=>{
   if(!active||current.current!==projectId)return;
   const found=fresh.find(project=>project.id===projectId);
   if(found){setProject(found);query.setQueryData(["projects"],fresh);}
   else nav("/");
  }).catch(error=>{if(active&&current.current===projectId)setError(errorText(error));});
  return()=>{active=false;};
 },[projectId,projects,nav,query,setProject,setError]);
}
