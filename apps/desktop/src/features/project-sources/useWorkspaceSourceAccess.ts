import {useCallback} from "react";
import {useWorkspace} from "../../stores/workspace";
import {isSourceCurrent,type ProjectSourceBinding} from "./sourceBinding";
import { useProjectAccess } from "../project-access/useProjectAccess";

/** A read-only folder selector must never acquire managed editing for its ID. */
export function useWorkspaceSourceAccess(projectId: string | undefined, source: ProjectSourceBinding | null, onError: (message: string) => void) {
  const report=useCallback((message:string)=>{
    const active=useWorkspace.getState().sourceBinding;
    if(source?isSourceCurrent(active,source):!active)onError(message);
  },[source,onError]);
  const managed = useProjectAccess(source?.kind === "folder" ? undefined : projectId, report,source?.openingId);
  return source?.kind === "folder" ? { ...managed, writable: false, checking: false, status: undefined, error: undefined } : managed;
}
