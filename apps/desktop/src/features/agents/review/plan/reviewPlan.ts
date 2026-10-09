import { z } from "zod";
import type { TypedCadDocument } from "../../../../lib/typedCadDocument";
import type { PreviewMetrics } from "../../../model-preview/previewPacket";
const id = z.string().regex(/^[A-Za-z0-9_-]{1,80}$/);
const text = (max: number) => z.string().refine(value => value.trim().length > 0 && Array.from(value).length <= max);
const positive = z.number().positive();
export const reviewPlanSchema = z.strictObject({
  assumptions: z.array(text(400)).max(16),
  dimensions: z.array(z.strictObject({ name: text(80), valueMm: z.number().min(0).max(10000), parameterId: id.optional() })).max(32),
  affectedBodyIds: z.array(id).max(32).refine(ids => new Set(ids).size === ids.length),
  expectedChecks: z.array(z.discriminatedUnion("type", [
    z.strictObject({ type: z.literal("validSolid") }),
    z.strictObject({ type: z.literal("bodyCount"), count: z.number().int().min(1).max(32) }),
    z.strictObject({ type: z.literal("bounds"), sizeMm: z.tuple([positive, positive, positive]), toleranceMm: z.number().min(0).max(10) }),
    z.strictObject({ type: z.literal("volume"), valueMm3: positive, toleranceMm3: z.number().nonnegative() }),
  ])).max(16),
});
export type ReviewPlan = z.infer<typeof reviewPlanSchema>;
export function checkReviewPlan(plan: ReviewPlan, document: TypedCadDocument | null, metrics: PreviewMetrics | undefined): boolean[] {
  return plan.expectedChecks.map(check => {
    if (!document || !metrics) return false;
    switch (check.type) {
      case "validSolid": return metrics.volumeMm3 > 0 && metrics.areaMm2 > 0 && metrics.faceCount > 0;
      case "bodyCount": return document.bodies.length === check.count;
      case "bounds": return metrics.boundsMm.every((value, index) => Math.abs(value - check.sizeMm[index]) <= check.toleranceMm);
      case "volume": return Math.abs(metrics.volumeMm3 - check.valueMm3) <= check.toleranceMm3;
    }
  });
}
