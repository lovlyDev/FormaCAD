import { useWorkspace } from "../../stores/workspace";
import { readProjectSnapshot } from "../project-catalog/api";
import { observeFolderCommit } from "./commitObservation";
import { isSourceCurrent, type ProjectSourceBinding } from "./sourceBinding";
import type { FolderWriteResult } from "./writeApi";

type Opening = Extract<ProjectSourceBinding, { kind: "folder" }>;
type UncertainWrite = Extract<FolderWriteResult, { status: "indeterminate" }>;

export function isFolderWriteOpeningCurrent(captured: Opening): boolean {
  const { project, sourceBinding } = useWorkspace.getState();
  return sourceBinding?.kind === "folder"
    && isSourceCurrent(sourceBinding, captured)
    && project?.id === captured.projectId
    && sourceBinding.source.commitId === captured.source.commitId;
}

function matchesOpening(captured: Opening, reply: FolderWriteResult): boolean {
  return isFolderWriteOpeningCurrent(captured)
    && reply.projectId === captured.projectId
    && reply.sourceId === captured.source.sourceId
    && reply.baseCommitId === captured.source.commitId;
}

/** Adopt confirmed writes and let the caller present all other outcomes. */
export function handleFolderWriteResult(
  captured: Opening,
  reply: FolderWriteResult,
): FolderWriteResult | { status: "stale" } {
  if (!matchesOpening(captured, reply)) return { status: "stale" };
  if (reply.status === "committed") {
    const next = { ...captured.source, commitId: reply.commitId };
    if (!useWorkspace.getState().publishFolderSnapshot(reply.project, captured, next)) {
      return { status: "stale" };
    }
  }
  return reply;
}

/** User-initiated lookup of an uncertain write, with no mutation on the host. */
export async function checkUncertainFolderWrite(captured: Opening, reply: UncertainWrite) {
  if (!matchesOpening(captured, reply)) return "stale" as const;
  const observation = await observeFolderCommit(captured.projectId, captured.source, reply.commitId);
  if (!matchesOpening(captured, reply)) return "stale" as const;
  if (observation.status !== "current") return observation.status;

  const next = { ...captured.source, commitId: reply.commitId };
  const project = await readProjectSnapshot(captured.projectId, next);
  if (!matchesOpening(captured, reply)) return "stale" as const;
  return useWorkspace.getState().publishFolderSnapshot(project, captured, next)
    ? "current" as const
    : "stale" as const;
}
