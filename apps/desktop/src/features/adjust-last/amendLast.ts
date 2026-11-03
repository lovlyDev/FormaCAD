import type { Project } from "../../types";
import {
  amendLastTransform,
  lastTransform,
  type CapturedLastTransform,
} from "./amendApi";
import {
  prepareAmendTransform,
  reopenLastTransform,
  type LastTransform,
  type LastTransformHead,
} from "./amendTransform";
import { sessionFromFields, type AdjustFields } from "./adjustFields";

const head = (captured: CapturedLastTransform): LastTransformHead => ({
  projectId: captured.projectId,
  revisionId: captured.definition.expectedRevision,
  sceneToken: `revision:${captured.definition.expectedRevision}`,
  transactionId: captured.definition.transactionId,
  generation: captured.definition.expectedGeneration,
});

/** Host-authoritative F9: re-reads the durable head, replaces matrices, keeps one Undo slot. */
export async function amendLast(
  project: Project,
  record: LastTransform,
  fields: AdjustFields,
): Promise<{ project: Project; record: LastTransform }> {
  const captured = await lastTransform(project.id);
  if (
    !captured ||
    captured.definition.expectedRevision !== project.currentRevision
  )
    throw new Error("AMEND_STALE");
  const current = head(captured);
  const reopened = reopenLastTransform(record, current);
  if (!reopened) throw new Error("AMEND_STALE");
  const { plan, error } = prepareAmendTransform(
    record,
    sessionFromFields(reopened, fields),
    current,
  );
  if (error) throw new Error(error);
  if (!plan) throw new Error("NO_CHANGE");
  const next = await amendLastTransform(
    project,
    captured,
    plan.replacements.map(({ commandIndex, matrix }) => ({
      commandIndex,
      matrix,
    })),
  );
  const after = await lastTransform(next.id).catch(() => null);
  return {
    project: next,
    record: after
      ? { ...record, committed: head(after) }
      : {
          ...record,
          committed: { ...current, revisionId: next.currentRevision ?? "" },
        },
  };
}
