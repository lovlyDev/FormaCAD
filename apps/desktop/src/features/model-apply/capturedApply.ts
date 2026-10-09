/** Capture the authoring context before permission/review; never retarget a pending program. */
export interface ApplyIdentity { id: string; currentRevision: string | null }
export interface CapturedApply {
  readonly base: Readonly<ApplyIdentity>;
  readonly program: string;
  readonly prompt: string;
}
export type Adoption = "orphaned" | "conflict" | "unchanged" | "alreadyAdopted" | "committed";

export function captureApply(base: ApplyIdentity, program: string, prompt: string): CapturedApply {
  return Object.freeze({ base: Object.freeze({ id: base.id, currentRevision: base.currentRevision }), program, prompt });
}

export function assertApplyBase(current: ApplyIdentity | null, captured: CapturedApply): void {
  if (!current || current.id !== captured.base.id || current.currentRevision !== captured.base.currentRevision)
    throw new Error("MODEL_APPLY_BASE_CHANGED");
}

/** UUIDs are identities, never temporal ordering; a conflicting head requires authoritative reconciliation. */
export function applyAdoption(current: ApplyIdentity | null, captured: CapturedApply, result: ApplyIdentity): Adoption {
  if (result.id !== captured.base.id) throw new Error("MODEL_APPLY_RESULT_INVALID");
  if (!current || current.id !== captured.base.id) return "orphaned";
  if (current.currentRevision === result.currentRevision) {
    return result.currentRevision === captured.base.currentRevision ? "unchanged" : "alreadyAdopted";
  }
  if (current.currentRevision !== captured.base.currentRevision) return "conflict";
  return "committed";
}

export interface ApplyRoute<P extends ApplyIdentity> {
  getCurrent(): P | null;
  invoke(request: CapturedApply): Promise<P>;
  adopt(result: P, disposition: "unchanged" | "alreadyAdopted" | "committed"): void;
  reconcileTarget(result: P, disposition: "orphaned" | "conflict"): Promise<void>;
  /** Must target the captured project; use authoritative targeted message append, not stale generic payload save. */
  saveCommittedMetadata?(result: P): Promise<void>;
  metadataFailed?(result: P, cause: unknown): void;
  refreshTarget(projectId: string): void;
}

/** Call inside approved callback, not when merely opening the confirmation dialog. */
export async function executeCapturedApply<P extends ApplyIdentity>(captured: CapturedApply, route: ApplyRoute<P>): Promise<{ result: P; disposition: Adoption; adoptionFailure?: unknown; metadataFailure?: unknown; refreshFailure?: unknown }> {
  assertApplyBase(route.getCurrent(), captured);
  const result = await route.invoke(captured);
  const disposition = applyAdoption(route.getCurrent(), captured, result);
  // Adopt durable geometry before any optional conversation save. No-op/alreadyAdopted adoption preserves selection.
  let adoptionFailure: unknown, metadataFailure: unknown, refreshFailure: unknown;
  try {
    try {
      if (disposition === "orphaned" || disposition === "conflict") await route.reconcileTarget(result, disposition);
      else route.adopt(result, disposition);
    } catch (cause) { adoptionFailure = cause; }
    if (route.saveCommittedMetadata) {
      try { await route.saveCommittedMetadata(result); }
      catch (cause) {
        metadataFailure = cause;
        // A presentation callback cannot turn an accepted native result into a failed geometry transaction.
        try { route.metadataFailed?.(result, cause); } catch (presentationCause) { adoptionFailure ??= presentationCause; }
      }
    }
  } finally {
    try { route.refreshTarget(captured.base.id); } catch (cause) { refreshFailure = cause; }
  }
  return { result, disposition,
    ...(adoptionFailure === undefined ? {} : { adoptionFailure }),
    ...(metadataFailure === undefined ? {} : { metadataFailure }),
    ...(refreshFailure === undefined ? {} : { refreshFailure }),
  };
}
