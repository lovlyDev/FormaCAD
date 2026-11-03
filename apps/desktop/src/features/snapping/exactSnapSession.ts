import { z } from "zod";
import { topologyReferenceSchema } from "../viewer/topology/topologyReference";
import {
  sameSelectionScope,
  type SelectionScope,
} from "../selection/selectionModel";
const point = z.tuple([
  z.number().finite().min(-1e9).max(1e9),
  z.number().finite().min(-1e9).max(1e9),
  z.number().finite().min(-1e9).max(1e9),
]);
const normal = z
  .tuple([z.number().finite(), z.number().finite(), z.number().finite()])
  .refine(
    (value) => Math.abs(value.reduce((sum, n) => sum + n * n, 0) - 1) < 1e-8,
  );
const assetSeal = z
  .strictObject({
    assetId: z.string().regex(/^step_[a-f0-9]{64}$/),
    sha256: z.string().regex(/^[a-f0-9]{64}$/),
    size: z
      .number()
      .int()
      .positive()
      .max(40 * 1024 * 1024),
  })
  .refine((seal) => seal.assetId === `step_${seal.sha256}`);
const assetSeals = z
  .array(assetSeal)
  .max(32)
  .refine(
    (seals) =>
      seals.reduce((sum, seal) => sum + seal.size, 0) <= 128 * 1024 * 1024 &&
      seals.every(
        (seal, index) => !index || seals[index - 1].assetId < seal.assetId,
      ),
  );
/** Contract for a verified native adapter; JSON shape checking alone is not a geometric proof. */
export const exactSnapSampleSchema = z.strictObject({
  requestId: z.string().uuid(),
  projectId: z.string().min(1).max(100),
  revisionId: z.string().min(1).max(100),
  sceneToken: z.string().min(1).max(100),
  sourceSha256: z.string().regex(/^[a-f0-9]{64}$/),
  sourceSize: z
    .number()
    .int()
    .positive()
    .max(40 * 1024 * 1024),
  documentSha256: z.string().regex(/^[a-f0-9]{64}$/),
  evaluationSource: z.literal("rebuiltAuthoredBody"),
  importedAssetSeals: assetSeals,
  bodyId: z.string().regex(/^[A-Za-z0-9_-]{1,80}$/),
  engine: z.literal("occt"),
  candidates: z
    .array(
      z
        .strictObject({
          kind: z.enum(["edgeMidpoint", "circleCenter", "face"]),
          reference: topologyReferenceSchema,
          pointMm: point,
          normal: normal.optional(),
        })
        .refine((value) =>
          value.kind === "face"
            ? value.reference.kind === "face" && !!value.normal
            : value.reference.kind === "edge",
        ),
    )
    .max(128),
});
export type ExactSnapSample = z.infer<typeof exactSnapSampleSchema>;
export interface SnapSourceCapture extends SelectionScope {
  sourceSha256: string;
  sourceSize: number;
  documentSha256: string;
  bodyId: string;
  importedAssetSeals: { assetId: string; sha256: string; size: number }[];
}
export interface SnapRequestCapture extends SnapSourceCapture {
  requestId: string;
}
function sameSource(
  left: SnapSourceCapture,
  right: SnapSourceCapture,
): boolean {
  return (
    sameSelectionScope(left, right) &&
    left.sourceSha256 === right.sourceSha256 &&
    left.sourceSize === right.sourceSize &&
    left.documentSha256 === right.documentSha256 &&
    left.bodyId === right.bodyId &&
    left.importedAssetSeals.length === right.importedAssetSeals.length &&
    left.importedAssetSeals.every(
      (seal, index) =>
        seal.assetId === right.importedAssetSeals[index].assetId &&
        seal.sha256 === right.importedAssetSeals[index].sha256 &&
        seal.size === right.importedAssetSeals[index].size,
    )
  );
}
export function readExactSnap(
  value: unknown,
  expected: SnapRequestCapture,
  current: SnapSourceCapture,
): ExactSnapSample | null {
  if (!sameSource(expected, current)) return null;
  if (
    !value ||
    typeof value !== "object" ||
    !("candidates" in value) ||
    !Array.isArray(value.candidates) ||
    value.candidates.length > 128 ||
    !("importedAssetSeals" in value) ||
    !Array.isArray(value.importedAssetSeals) ||
    value.importedAssetSeals.length > 32
  )
    return null;
  const result = exactSnapSampleSchema.safeParse(value);
  if (!result.success) return null;
  const report = result.data;
  if (!sameSource(report, expected) || report.requestId !== expected.requestId)
    return null;
  return report;
}
