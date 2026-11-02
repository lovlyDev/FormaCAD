import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import {
  topologyReferenceSchema,
  type EdgeTopologyReference,
  type FaceTopologyReference,
} from "../viewer/topology/topologyReference";
import type { Vec3 } from "../transform/frames";

export type ExactSnapQuery =
  | { kind: "edgeMidpoint"; reference: EdgeTopologyReference }
  | { kind: "circleCenter"; reference: EdgeTopologyReference }
  | {
      kind: "face";
      reference: FaceTopologyReference;
      rayOriginMm: Vec3;
      rayDirection: Vec3;
    };

const coordinate = z.number().finite().min(-1e9).max(1e9);
const point = z.tuple([coordinate, coordinate, coordinate]);
const confirmed = z.object({
  projectId: z.string(),
  revisionId: z.string(),
  bodyId: z.string(),
  engine: z.object({ name: z.literal("occt") }).passthrough(),
  evaluationSource: z.literal("rebuiltAuthoredBody"),
  query: z
    .object({
      kind: z.enum(["edgeMidpoint", "circleCenter", "face"]),
      reference: topologyReferenceSchema,
    })
    .passthrough(),
  result: z.strictObject({
    pointMm: point,
    normal: point
      .refine((value) => Math.abs(Math.hypot(...value) - 1) < 1e-8)
      .optional(),
  }),
});

type Reference = z.infer<typeof topologyReferenceSchema>;
const sameReference = (left: Reference, right: Reference) =>
  left.kind === right.kind &&
  left.role === right.role &&
  left.ownerFeatureId === right.ownerFeatureId &&
  left.occurrencePath.length === right.occurrencePath.length &&
  left.occurrencePath.every((id, index) => id === right.occurrencePath[index]);

export interface ExactSnapResult {
  pointMm: Vec3;
  normal: Vec3 | null;
}

/**
 * Exact OCCT snap on the committed authored body. The host rebuilds the body in
 * an isolated worker; this adapter additionally checks the echo belongs to the
 * requested project, revision, body and query before trusting the point.
 */
export async function exactSnap(
  scope: { projectId: string; revisionId: string; bodyId: string },
  query: ExactSnapQuery,
): Promise<ExactSnapResult> {
  const raw: unknown = await invoke("snap_model_reference", {
    projectId: scope.projectId,
    expectedRevision: scope.revisionId,
    bodyId: scope.bodyId,
    query,
  });
  const parsed = confirmed.safeParse(raw);
  if (
    !parsed.success ||
    parsed.data.projectId !== scope.projectId ||
    parsed.data.revisionId !== scope.revisionId ||
    parsed.data.bodyId !== scope.bodyId ||
    parsed.data.query.kind !== query.kind ||
    !sameReference(parsed.data.query.reference, query.reference) ||
    (query.kind === "face") !== !!parsed.data.result.normal
  )
    throw new Error("SNAP_REPORT_INVALID");
  return {
    pointMm: parsed.data.result.pointMm,
    normal: parsed.data.result.normal ?? null,
  };
}
