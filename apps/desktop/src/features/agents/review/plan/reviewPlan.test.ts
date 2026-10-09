import { expect, it } from "vitest";
import { checkReviewPlan, reviewPlanSchema, type ReviewPlan } from "./reviewPlan";
import type { TypedCadDocument } from "../../../../lib/typedCadDocument";
const plan: ReviewPlan = { assumptions: ["closed part"], dimensions: [], affectedBodyIds: ["body"], expectedChecks: [{ type: "validSolid" }, { type: "bounds", sizeMm: [40, 20, 10], toleranceMm: 0.001 }, { type: "volume", valueMm3: 8000, toleranceMm3: 0.01 }, { type: "bodyCount", count: 1 }] };
const document = { bodies: [{ id: "body" }] } as TypedCadDocument;
const metrics = { volumeMm3: 8000, areaMm2: 2800, boundsMm: [40, 20, 10] as [number, number, number], faceCount: 6, edgeCount: 12 };
it("checks expected geometry against independent measured metrics rather than accepting AI claims", () => {
  expect(checkReviewPlan(plan, document, metrics)).toEqual([true, true, true, true]);
  expect(checkReviewPlan(plan, document, { ...metrics, volumeMm3: 7000, boundsMm: [35, 20, 10] })).toEqual([true, false, false, true]);
  expect(checkReviewPlan(plan, document, undefined)).toEqual([false, false, false, false]);
});
it("bounds and validates the structured plan without accepting extra fields", () => {
  expect(reviewPlanSchema.safeParse(plan).success).toBe(true);
  expect(reviewPlanSchema.safeParse({ ...plan, arbitraryCommand: "run" }).success).toBe(false);
  expect(reviewPlanSchema.safeParse({ ...plan, affectedBodyIds: ["body", "body"] }).success).toBe(false);
  expect(reviewPlanSchema.safeParse({ ...plan, expectedChecks: [{ type: "volume", valueMm3: Infinity, toleranceMm3: 0 }] }).success).toBe(false);
});
