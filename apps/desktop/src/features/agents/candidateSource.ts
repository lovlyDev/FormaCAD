import type { Project } from "../../types";
type Source = Pick<Project, "id" | "currentRevision">;
/** Revision IDs alone do not identify a project: copied projects may share history. */
export function candidateSourceStatus(current: Source | null, base: Source) {
  if (!current || current.id !== base.id) return "projectChanged";
  return current.currentRevision === base.currentRevision ? "valid" : "revisionChanged";
}
