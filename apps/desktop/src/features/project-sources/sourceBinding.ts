import type { FolderSourceRef } from "./contract";

// Renderer request identity only. Host source references remain independently
// validated; this local UUID never grants storage or writer access.
interface Opening { readonly openingId: string; readonly projectId: string }
export type ProjectSourceBinding =
  | Readonly<Opening & { kind: "managed" }>
  | Readonly<Opening & { kind: "folder"; source: Readonly<FolderSourceRef> }>;

const changed = () => new Error("PROJECT_SNAPSHOT_CHANGED");
function folderRef(source: FolderSourceRef): Readonly<FolderSourceRef> {
  if (!source.sourceId || !source.commitId || !source.taskScopeId) throw changed();
  return Object.freeze({ sourceId: source.sourceId, commitId: source.commitId, taskScopeId: source.taskScopeId });
}

/** Reuse only for metadata publication within the same managed opening. */
export function managedSource(projectId: string, previous?: ProjectSourceBinding | null): ProjectSourceBinding {
  if (!projectId) throw changed();
  if (previous?.kind === "managed" && previous.projectId === projectId) return previous;
  return Object.freeze({ kind: "managed", projectId, openingId: crypto.randomUUID() });
}

/** Explicit activation always starts a new renderer request family. */
export function folderSource(projectId: string, source: FolderSourceRef): ProjectSourceBinding {
  if (!projectId) throw changed();
  return Object.freeze({ kind: "folder", projectId, openingId: crypto.randomUUID(), source: folderRef(source) });
}

/** Advance a validated commit reference without rebinding the original opening. */
export function refreshFolderSource(current: ProjectSourceBinding, source: FolderSourceRef): ProjectSourceBinding {
  const next = folderRef(source);
  if (current.kind !== "folder" || current.source.sourceId !== next.sourceId || current.source.taskScopeId !== next.taskScopeId) throw changed();
  return Object.freeze({ ...current, source: next });
}

/** Metadata-only refresh remains compatible; geometry consumers compare their seals too. */
export function isSourceCurrent(active: ProjectSourceBinding | null, captured: ProjectSourceBinding): boolean {
  return !!active && active.openingId === captured.openingId && active.projectId === captured.projectId &&
    active.kind === captured.kind && (active.kind === "managed" || (captured.kind === "folder" &&
      active.source.sourceId === captured.source.sourceId && active.source.taskScopeId === captured.source.taskScopeId));
}

/** A mismatched binding must never become an undefined (managed) fallback. */
export function sourceRefFor(binding: ProjectSourceBinding, projectId: string): Readonly<FolderSourceRef> | undefined {
  if (binding.projectId !== projectId) throw changed();
  return binding.kind === "folder" ? binding.source : undefined;
}
