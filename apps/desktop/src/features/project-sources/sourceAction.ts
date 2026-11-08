import { useWorkspace } from "../../stores/workspace";
import { isSourceCurrent, type ProjectSourceBinding } from "./sourceBinding";

/** Bind an existing managed action before any permission/agent await. */
export function managedActionSource(projectId: string, captured: ProjectSourceBinding | null) {
  const isCurrent = () => {
    const active = useWorkspace.getState();
    return !!captured && captured.projectId === projectId && active.project?.id === projectId &&
      isSourceCurrent(active.sourceBinding, captured);
  };
  return {
    openingId: captured?.openingId,
    isCurrent,
    assertCurrent() {
      if (!isCurrent()) throw new Error("PROJECT_SNAPSHOT_CHANGED");
      if (captured?.kind === "folder") throw new Error("PROJECT_SOURCE_WRITE_NOT_AUTHORIZED");
    },
  };
}
