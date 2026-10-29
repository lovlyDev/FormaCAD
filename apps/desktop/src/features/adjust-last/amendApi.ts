import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type { Project } from "../../types";
const id = z.string().regex(/^[A-Za-z0-9_-]{1,80}$/);
const matrix = z
  .array(z.number().finite())
  .length(12)
  .refine((values) => {
    if (
      values.some(
        (value, index) => Math.abs(value) > (index % 4 === 3 ? 10000 : 1000),
      )
    )
      return false;
    const determinant =
      values[0] * (values[5] * values[10] - values[6] * values[9]) -
      values[1] * (values[4] * values[10] - values[6] * values[8]) +
      values[2] * (values[4] * values[9] - values[5] * values[8]);
    return Number.isFinite(determinant) && determinant > 1e-18;
  });
const command = z.strictObject({
  commandIndex: z.number().int().min(0).max(255),
  bodyId: id,
  featureId: id,
  name: z.string().max(200),
  matrix,
});
const definition = z
  .strictObject({
    expectedRevision: z.string().uuid(),
    expectedGeneration: z.number().int().min(0).max(Number.MAX_SAFE_INTEGER),
    transactionId: z.string().uuid(),
    baseRevision: z.string().uuid(),
    commands: z.array(command).min(1).max(256),
  })
  .refine(
    (value) =>
      value.transactionId === value.expectedRevision &&
      value.commands.every((item, index) => item.commandIndex === index) &&
      new Set(value.commands.map((item) => item.bodyId)).size ===
        value.commands.length &&
      new Set(value.commands.map((item) => item.featureId)).size ===
        value.commands.length,
  );
export interface CapturedLastTransform {
  projectId: string;
  definition: z.infer<typeof definition>;
}
export async function lastTransform(
  projectId: string,
): Promise<CapturedLastTransform | null> {
  if (!z.string().uuid().safeParse(projectId).success)
    throw new Error("AMEND_STALE");
  const value: unknown = await invoke("last_transform", { projectId });
  if (value === null) return null;
  const result = definition.safeParse(value);
  if (!result.success)
    throw new Error("AMEND_REPORT_INVALID", { cause: result.error });
  return { projectId, definition: result.data };
}
const replacementsSchema = z
  .array(
    z.strictObject({ commandIndex: z.number().int().min(0).max(255), matrix }),
  )
  .min(1)
  .max(256)
  .refine(
    (items) =>
      new Set(items.map((item) => item.commandIndex)).size === items.length,
  );

/** Capture ownership is checked before IPC; host independently checks its durable head/permissions. */
export async function amendLastTransform(
  current: Pick<Project, "id" | "currentRevision">,
  captured: CapturedLastTransform,
  replacements: { commandIndex: number; matrix: number[] }[],
): Promise<Project> {
  if (
    current.id !== captured.projectId ||
    current.currentRevision !== captured.definition.expectedRevision
  )
    throw new Error("AMEND_STALE");
  const parsed = replacementsSchema.safeParse(replacements);
  if (!parsed.success)
    throw new Error("TRANSFORM_INPUT_INVALID", { cause: parsed.error });
  if (
    parsed.data.some(
      (item) => item.commandIndex >= captured.definition.commands.length,
    )
  )
    throw new Error("AMEND_STALE");
  return invoke("amend_last_transform", {
    projectId: captured.projectId,
    expectedRevision: captured.definition.expectedRevision,
    expectedGeneration: captured.definition.expectedGeneration,
    transactionId: captured.definition.transactionId,
    replacements: parsed.data,
  });
}
