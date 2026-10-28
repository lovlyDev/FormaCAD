import { z } from "zod";
import { topologyReferenceSchema } from "../viewer/topology/topologyReference";
import {
  sameSelectionScope,
  type SelectionScope,
} from "../selection/selectionModel";
const point = z.tuple([
  z.number().finite().min(-10000).max(10000),
  z.number().finite().min(-10000).max(10000),
  z.number().finite().min(-10000).max(10000),
]);
const normal = z
  .tuple([z.number().finite(), z.number().finite(), z.number().finite()])
  .refine(
    (value) => Math.abs(value.reduce((sum, n) => sum + n * n, 0) - 1) < 1e-8,
  );
/** Renderer boundary for a future native snap adapter; mesh intersections cannot satisfy this contract. */
export const exactSnapSampleSchema = z.strictObject({
  requestId: z.string().uuid(),
  projectId: z.string().min(1).max(100),
  revisionId: z.string().min(1).max(100),
  sceneToken: z.string().min(1).max(100),
  sourceSha256: z.string().regex(/^[a-f0-9]{64}$/),
  documentSha256: z.string().regex(/^[a-f0-9]{64}$/),
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
export interface SnapRequestCapture extends SelectionScope {
  requestId: string;
  sourceSha256: string;
  documentSha256: string;
  bodyId: string;
}
export function readExactSnap(
  value: unknown,
  expected: SnapRequestCapture,
  current: SelectionScope,
): ExactSnapSample | null {
  if (!sameSelectionScope(expected, current)) return null;
  if (
    !value ||
    typeof value !== "object" ||
    !("candidates" in value) ||
    !Array.isArray(value.candidates) ||
    value.candidates.length > 128
  )
    return null;
  const result = exactSnapSampleSchema.safeParse(value);
  if (!result.success) return null;
  const report = result.data;
  if (
    !sameSelectionScope(report, expected) ||
    report.requestId !== expected.requestId ||
    report.bodyId !== expected.bodyId ||
    report.sourceSha256 !== expected.sourceSha256 ||
    report.documentSha256 !== expected.documentSha256
  )
    return null;
  return report;
}
