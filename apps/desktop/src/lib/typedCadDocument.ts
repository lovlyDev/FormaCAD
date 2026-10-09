import { z } from "zod";

const id = z.string().regex(/^[A-Za-z0-9_-]{1,80}$/);
const operation = z.looseObject({ type: z.string() });
const feature = z.looseObject({
  id,
  name: z.string(),
  operation,
  suppressed: z.boolean().optional(),
});
const body = z.looseObject({ id, name: z.string(), sourceFeatureId: id });
const documentSchema = z.looseObject({
  schemaVersion: z.literal(2),
  revisionId: id,
  parameters: z.array(z.looseObject({ id, name: z.string(), valueMm: z.number() })),
  features: z.array(feature),
  bodies: z.array(body),
});

export type TypedCadDocument = z.infer<typeof documentSchema>;
export type TypedCadFeature = TypedCadDocument["features"][number];

export function readTypedCadDocument(source: string): TypedCadDocument | null {
  try {
    const parsed = documentSchema.safeParse(JSON.parse(source));
    return parsed.success ? parsed.data : null;
  } catch {
    return null;
  }
}

export function featureDependencies(feature: TypedCadFeature): string[] {
  const op = feature.operation;
  switch (op.type) {
    case "extrude":
    case "revolve": return typeof op.sketchId === "string" ? [op.sketchId] : [];
    case "hole":
    case "fillet":
    case "filletEdge":
    case "filletReferencedEdge":
    case "chamfer":
    case "translate":
    case "rotate":
    case "mirror":
      return typeof op.bodyFeatureId === "string" ? [op.bodyFeatureId] : [];
    case "boolean":
      return [op.leftFeatureId, op.rightFeatureId].filter((value): value is string => typeof value === "string");
    default: return [];
  }
}

export function canSuppressFeature(feature: TypedCadFeature): boolean {
  return ["hole", "fillet", "filletEdge", "filletReferencedEdge", "chamfer", "translate", "rotate", "mirror"].includes(feature.operation.type);
}

export function solidFeature(feature: TypedCadFeature): boolean {
  return ["importStep", "sphere", "cone", "extrude", "revolve", "hole", "fillet", "filletEdge", "filletReferencedEdge", "chamfer", "translate", "rotate", "mirror", "boolean"].includes(feature.operation.type);
}

export function activeFeatureIds(document: TypedCadDocument): Set<string> {
  const active = new Set(document.bodies.map((body) => body.sourceFeatureId));
  for (const feature of [...document.features].reverse()) {
    if (active.has(feature.id)) featureDependencies(feature).forEach((id) => active.add(id));
  }
  return active;
}
