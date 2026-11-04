import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { t } from "../../../i18n";
import { readTypedCadDocument } from "../../../lib/typedCadDocument";
import type { Project } from "../../../types";
import { captureApply } from "../../model-apply/capturedApply";

const coordinate = z.number().min(-10000).max(10000);
const origin = z.tuple([coordinate, coordinate, coordinate]);
const component = z.number().min(-1e6).max(1e6);
const direction = z.tuple([component, component, component])
  .refine((values) => Math.hypot(...values) > 1e-7);
const span = z.number().gt(1e-7).max(10000);
const operationSchema = z.discriminatedUnion("type", [
  z.strictObject({ type: z.literal("datumPlane"), originMm: origin, normal: direction, sideMm: span }),
  z.strictObject({ type: z.literal("datumAxis"), originMm: origin, direction, lengthMm: span }),
  z.strictObject({ type: z.literal("datumPoint"), originMm: origin }),
]);

export type DatumOperation = z.infer<typeof operationSchema>;
export type DatumKind = DatumOperation["type"];
export const datumLabels = {
  datumPlane: "Datum plane",
  datumAxis: "Datum axis",
  datumPoint: "Datum point",
} as const satisfies Record<DatumKind, string>;

export function readDatumOperation(value: unknown): DatumOperation | null {
  const parsed = operationSchema.safeParse(value);
  return parsed.success ? parsed.data : null;
}

export function projectDatums(program: string | null | undefined) {
  const document = program && readTypedCadDocument(program);
  if (!document) return [];
  return document.features.flatMap((feature) => {
    const datum = readDatumOperation(feature.operation);
    return datum && !feature.suppressed ? [{ id: feature.id, datum }] : [];
  });
}

export function captureDatum(project: Project, operation: DatumOperation) {
  const name = t(datumLabels[operation.type]);
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
