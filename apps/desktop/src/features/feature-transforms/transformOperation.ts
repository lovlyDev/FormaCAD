import { z } from "zod";
const origin = z.tuple([z.number(), z.number(), z.number()]).refine(value => value.every(n => Number.isFinite(n) && Math.abs(n) <= 10000));
// A zero direction remains editable in a draft; native validation rejects saving it.
const direction = z.tuple([z.number(), z.number(), z.number()]).refine(value => value.every(n => Number.isFinite(n) && Math.abs(n) <= 1e6));
const transform = z.union([
  z.looseObject({ type: z.literal("rotate"), bodyFeatureId: z.string(), axisOriginMm: origin, axisDirection: direction, angleDeg: z.number().min(-360).max(360) }),
  z.looseObject({ type: z.literal("revolve"), sketchId: z.string(), axisOriginMm: origin, axisDirection: direction, angleDeg: z.number().min(-360).max(360) }),
  z.looseObject({ type: z.literal("mirror"), bodyFeatureId: z.string(), planeOriginMm: origin, planeNormal: direction }),
]);
export type TransformOperation = z.infer<typeof transform>;
export function readTransformOperation(value: unknown): TransformOperation | null {
  const parsed = transform.safeParse(value);
  return parsed.success ? parsed.data : null;
}
