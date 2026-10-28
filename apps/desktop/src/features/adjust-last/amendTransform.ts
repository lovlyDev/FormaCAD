import { prepareTransformCommands } from "../transform/transformCommands";
import {
  captureMatches,
  type TransformCapture,
  type TransformSession,
} from "../transform/transformSession";
export interface LastTransformHead extends TransformCapture {
  transactionId: string;
  generation: number;
}
export interface LastTransform {
  base: TransformSession;
  committed: LastTransformHead;
  featureIds: string[];
  commandIndices: number[];
  name: string;
}
export interface AmendTransformPlan {
  expectedRevision: string;
  expectedGeneration: number;
  transactionId: string;
  baseRevision: string;
  replacements: { commandIndex: number; featureId: string; matrix: number[] }[];
}
/** Transient session record, not a durable host authorization or a working amend endpoint. */
export function rememberLastTransform(
  base: TransformSession,
  committed: LastTransformHead,
  featureIds: readonly string[],
  commandIndices: readonly number[],
  name: string,
): LastTransform | null {
  if (
    base.capture.projectId !== committed.projectId ||
    base.capture.revisionId === committed.revisionId ||
    !committed.transactionId ||
    !Number.isSafeInteger(committed.generation) ||
    committed.generation < 1 ||
    featureIds.length !== base.targets.length ||
    commandIndices.length !== base.targets.length ||
    new Set(commandIndices).size !== commandIndices.length ||
    commandIndices.some(
      (index) => !Number.isInteger(index) || index < 0 || index >= 256,
    )
  )
    return null;
  const result = prepareTransformCommands(
    { ...base, phase: "preview" },
    base.capture,
    featureIds,
    name,
  );
  if (result.error || !result.commands.length) return null;
  return {
    base: structuredClone(base),
    committed: { ...committed },
    featureIds: [...featureIds],
    commandIndices: [...commandIndices],
    name,
  };
}
function headMatches(record: LastTransform, head: LastTransformHead): boolean {
  return (
    record.committed.projectId === head.projectId &&
    record.committed.revisionId === head.revisionId &&
    record.committed.transactionId === head.transactionId &&
    record.committed.generation === head.generation
  );
}
/** Original before-transform matrices are retained so 10 -> 20 mm replaces rather than stacks. */
export function reopenLastTransform(
  record: LastTransform,
  head: LastTransformHead,
): TransformSession | null {
  if (!headMatches(record, head)) return null;
  return {
    ...structuredClone(record.base),
    phase: "preview",
    capture: {
      projectId: head.projectId,
      revisionId: head.revisionId,
      sceneToken: head.sceneToken,
    },
  };
}
export function prepareAmendTransform(
  record: LastTransform,
  session: TransformSession,
  head: LastTransformHead,
): {
  plan: AmendTransformPlan | null;
  error?: "AMEND_STALE" | "TRANSFORM_INPUT_INVALID";
} {
  if (!headMatches(record, head) || !captureMatches(session.capture, head))
    return { plan: null, error: "AMEND_STALE" };
  const result = prepareTransformCommands(
    session,
    head,
    record.featureIds,
    record.name,
    "amend",
  );
  if (result.error)
    return {
      plan: null,
      error: result.error === "TRANSFORM_STALE" ? "AMEND_STALE" : result.error,
    };
  if (!result.commands.length) return { plan: null };
  return {
    plan: {
      expectedRevision: head.revisionId,
      expectedGeneration: head.generation,
      transactionId: head.transactionId,
      baseRevision: record.base.capture.revisionId,
      replacements: result.commands.map((command, index) => ({
        commandIndex: record.commandIndices[index],
        featureId: command.featureId,
        matrix: [...command.matrix],
      })),
    },
  };
}
