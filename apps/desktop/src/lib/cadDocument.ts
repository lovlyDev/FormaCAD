import { z } from "zod";

const dimension = z.number().finite().positive().max(10000);
const operation = z.discriminatedUnion("type", [
  z.strictObject({
    type: z.literal("rectangle"),
    width: dimension,
    depth: dimension,
  }),
  z.strictObject({ type: z.literal("circle"), radius: dimension }),
  z.strictObject({
    type: z.literal("extrude"),
    sketch: z.string(),
    distance: z
      .number()
      .finite()
      .min(-10000)
      .max(10000)
      .refine((n) => n !== 0),
  }),
  z.strictObject({
    type: z.literal("translate"),
    body: z.string(),
    offset: z.tuple([
      z.number().finite().min(-10000).max(10000),
      z.number().finite().min(-10000).max(10000),
      z.number().finite().min(-10000).max(10000),
    ]),
  }),
  z.strictObject({
    type: z.literal("boolean"),
    left: z.string(),
    right: z.string(),
    mode: z.enum(["union", "cut", "intersect"]),
  }),
]);
export const cadDocumentSchema = z.strictObject({
  version: z.literal(1),
  features: z
    .array(
      z.strictObject({
        id: z.string().regex(/^[A-Za-z0-9_-]{1,80}$/),
        operation,
      }),
    )
    .min(1)
    .max(128),
  output: z.string(),
});
export type CadDocument = z.infer<typeof cadDocumentSchema>;
export type CadOperation = CadDocument["features"][number]["operation"];

export function readCadDocument(source: string): CadDocument | null {
  try {
    const result = cadDocumentSchema.safeParse(JSON.parse(source));
    return result.success ? result.data : null;
  } catch {
    return null;
  }
}

export function dependencies(op: CadOperation): string[] {
  switch (op.type) {
    case "extrude":
      return [op.sketch];
    case "translate":
      return [op.body];
    case "boolean":
      return [op.left, op.right];
    default:
      return [];
  }
}
