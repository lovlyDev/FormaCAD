import { useWorkspace } from "../../stores/workspace";
import type { Vec3 } from "../transform/frames";
import { exactSnap } from "./snapApi";
import type { SnapCandidate } from "./snapCandidate";
import { useSnapSettings } from "./cursor/snapSettings";

export interface ResolvedSnap {
  pointMm: Vec3;
  exact: boolean;
}

/**
 * Refines a candidate with OCCT when exact snapping is on and the target has a
 * semantic reference. Returns null when the revision changed meanwhile, so a
 * late answer never lands on another model.
 */
export async function resolveSnap(
  candidate: SnapCandidate,
): Promise<ResolvedSnap | null> {
  const project = useWorkspace.getState().project;
  const settings = useSnapSettings.getState();
  if (!settings.exact || !candidate.query || !project?.currentRevision)
    return { pointMm: candidate.pointMm, exact: false };
  const revisionId = project.currentRevision;
  settings.setPending(true);
  try {
    const result = await exactSnap(
      { projectId: project.id, revisionId, bodyId: candidate.bodyId },
      candidate.query,
    );
    if (useWorkspace.getState().project?.currentRevision !== revisionId)
      return null;
    useSnapSettings.getState().setError(null);
    return { pointMm: result.pointMm, exact: true };
  } catch (cause) {
    useSnapSettings.getState().setError(cause);
    return { pointMm: candidate.pointMm, exact: false };
  } finally {
    useSnapSettings.getState().setPending(false);
  }
}
