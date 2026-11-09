import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { t } from "../../../i18n";
import { readTypedCadDocument } from "../../../lib/typedCadDocument";
import type { Project } from "../../../types";
import { captureApply } from "../../model-apply/capturedApply";
import { topologyReferenceSchema } from "../../viewer/topology/topologyReference";

const coordinate = z.number().min(-10000).max(10000);
const origin = z.tuple([coordinate, coordinate, coordinate]);
const component = z.number().min(-1e6).max(1e6);
const direction = z.tuple([component, component, component])
  .refine((values) => Math.hypot(...values) > 1e-7);
const span = z.number().gt(1e-7).max(10000);
const absoluteSchema = z.discriminatedUnion("type", [
  z.strictObject({ type: z.literal("datumPlane"), originMm: origin, normal: direction, sideMm: span }),
  z.strictObject({ type: z.literal("datumAxis"), originMm: origin, direction, lengthMm: span }),
  z.strictObject({ type: z.literal("datumPoint"), originMm: origin }),
]);
const id = z.string().regex(/^[A-Za-z0-9_-]{1,80}$/);
const sourceSchema = z.discriminatedUnion("type", [
  z.strictObject({ type: z.literal("topology"), bodyId: id, reference: topologyReferenceSchema.refine((item) => item.kind !== "vertex") }),
  z.strictObject({ type: z.literal("plane"), featureId: id }),
]);
const relativeSchema = z.strictObject({
  type: z.literal("datumOnReference"),
  geometry: z.discriminatedUnion("type", [
    z.strictObject({ type: z.literal("plane"), sideMm: span }),
    z.strictObject({ type: z.literal("axis"), lengthMm: span }),
    z.strictObject({ type: z.literal("point") }),
  ]),
  anchor: z.strictObject({ source: sourceSchema, offsetMm: coordinate, angleDeg: z.number().min(-360).max(360) }),
});
const operationSchema = z.union([absoluteSchema, relativeSchema]);

export type DatumOperation = z.infer<typeof operationSchema>;
export type AbsoluteDatum = z.infer<typeof absoluteSchema>;
export type DatumKind = AbsoluteDatum["type"];
export type DatumSource = z.infer<typeof sourceSchema>;
export const datumKind = (datum: DatumOperation): DatumKind =>
  datum.type === "datumOnReference"
    ? { plane: "datumPlane", axis: "datumAxis", point: "datumPoint" }[datum.geometry.type] as DatumKind
    : datum.type;
export const datumLabels = {
  datumPlane: "Datum plane",
  datumAxis: "Datum axis",
  datumPoint: "Datum point",
} as const satisfies Record<DatumKind, string>;

export function readDatumOperation(value: unknown): DatumOperation | null {
  const parsed = operationSchema.safeParse(value);
  return parsed.success ? parsed.data : null;
}

export function isDatumOnlyProgram(program: string | null | undefined) {
  const document = program && readTypedCadDocument(program);
  return !!document && document.bodies.length === 0 && document.features.length > 0
    && document.features.every((feature) => !feature.suppressed && readDatumOperation(feature.operation));
}

export function captureDatum(project: Project, operation: DatumOperation) {
  const name = t(datumLabels[datumKind(operation)]);
  const feature = { id: crypto.randomUUID(), name, operation };
  const captured = captureApply(project, "", name);
  return {
    captured,
    invoke: () => invoke<Project>("apply_ir_commands", {
      projectId: captured.base.id,
      expectedRevision: captured.base.currentRevision,
      prompt: captured.prompt,
      commands: [{ command: "add_feature", feature }],
    }),
  };
}
