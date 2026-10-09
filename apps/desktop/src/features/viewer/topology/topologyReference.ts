import { z } from "zod";

const featureId = z.string().regex(/^[A-Za-z0-9_-]{1,80}$/);
export const edgeRoles = [
  "box-edge:x:ymin:zmin", "box-edge:x:ymin:zmax", "box-edge:x:ymax:zmin", "box-edge:x:ymax:zmax",
  "box-edge:y:xmin:zmin", "box-edge:y:xmin:zmax", "box-edge:y:xmax:zmin", "box-edge:y:xmax:zmax",
  "box-edge:z:xmin:ymin", "box-edge:z:xmin:ymax", "box-edge:z:xmax:ymin", "box-edge:z:xmax:ymax",
  "cylinder-edge:bottom", "cylinder-edge:top",
] as const;
const faceRoles = ["box-face:xmin", "box-face:xmax", "box-face:ymin", "box-face:ymax", "box-face:zmin", "box-face:zmax", "cylinder-face:bottom", "cylinder-face:top", "cylinder-face:side"] as const;
const common = { schemaVersion: z.literal(1), ownerFeatureId: featureId, occurrencePath: z.array(featureId).max(64) };
export const topologyReferenceSchema = z.discriminatedUnion("kind", [
  z.strictObject({ ...common, kind: z.literal("edge"), role: z.enum(edgeRoles) }),
  z.strictObject({ ...common, kind: z.literal("face"), role: z.enum(faceRoles) }),
]).refine(reference => new Set(reference.occurrencePath).size === reference.occurrencePath.length && !reference.occurrencePath.includes(reference.ownerFeatureId));
export type TopologyReference = z.infer<typeof topologyReferenceSchema>;
export type EdgeTopologyReference = Extract<TopologyReference, { kind: "edge" }>;
export type FaceTopologyReference = Extract<TopologyReference, { kind: "face" }>;
export function readTopologyReference(value: unknown): TopologyReference | null {
  const parsed = topologyReferenceSchema.safeParse(value);
  return parsed.success ? parsed.data : null;
}
