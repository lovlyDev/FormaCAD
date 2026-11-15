import { useEffect, useRef } from "react";
import { errorText, t } from "../../i18n";
import { native } from "../../lib/api";
import type { Project } from "../../types";
import { isIgesName, validIgesFile } from "./igesApi";
import { useIgesImportState } from "./igesImportState";
export function useIgesImport({ project, routeProjectId, blocked, onNew, onOpen, onClose, onError }: {
  project: Project | null; routeProjectId: string | null; blocked: boolean; onNew: () => void; onOpen: () => void; onClose: () => void; onError: (message: string) => void;
}) {
  const { pending, intent } = useIgesImportState();
  const live = useRef({ onOpen, onClose }); live.current = { onOpen, onClose };
  useEffect(() => {
    if(intent?.awaitingRoute){
      // Home may briefly clear the workspace while HashRouter commits navigation.
      // Open only under the explicitly created project's URL and loaded snapshot.
      if(routeProjectId===null || (routeProjectId===intent.project.id && !project))return;
      if(routeProjectId===intent.project.id && project?.id===intent.project.id && project.currentRevision===intent.project.currentRevision){
        useIgesImportState.setState({intent:{...intent,awaitingRoute:false}});return;
      }
      useIgesImportState.setState({intent:null});live.current.onClose();return;
    }
    if (intent && (project?.id !== intent.project.id || project.currentRevision !== intent.project.currentRevision)) {
      useIgesImportState.setState({intent:null}); live.current.onClose();
    } else if(intent) live.current.onOpen();
  }, [project?.id, project?.currentRevision, routeProjectId, intent]);
  return {
    intent,
    pendingName:pending?.name,
    intercept(files: FileList) {
      const chosen = Array.from(files);
      if (!chosen.some(file => isIgesName(file.name))) return false;
      if (!native) { onError(errorText("IGES_NATIVE_REQUIRED")); return true; }
      if (blocked) { onError(t("A project task is already running")); return true; }
      if (chosen.length !== 1) { onError(t("Import one IGES file at a time, without other attachments.")); return true; }
      const file = chosen[0];
      if (!validIgesFile(file)) { onError(errorText("IGES_INVALID_INPUT")); return true; }
      if (project) { useIgesImportState.setState({intent:{ id:crypto.randomUUID(), project, file }}); } else { useIgesImportState.setState({pending:file}); onNew(); }
      return true;
    },
    cancelPending() { useIgesImportState.setState({pending:null}); },
    startCreated(project: Project) {
      const file = useIgesImportState.getState().pending;
      if (!file) return false;
      useIgesImportState.setState({pending:null,intent:{ id:crypto.randomUUID(), project, file,awaitingRoute:true }}); return true;
    },
    close() { useIgesImportState.setState({intent:null}); onClose(); },
  };
}
