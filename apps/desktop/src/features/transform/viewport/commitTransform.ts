import { invoke } from "@tauri-apps/api/core";
import type { Project } from "../../../types";
import { readTypedCadDocument } from "../../../lib/typedCadDocument";
import { lastTransform } from "../../adjust-last/amendApi";
import {
  rememberLastTransform,
  type LastTransform,
} from "../../adjust-last/amendTransform";
import { prepareTransformCommands } from "../transformCommands";
import type { TransformCapture, TransformSession } from "../transformSession";

export const transformName: Record<TransformSession["kind"], string> = {
  move: "Move",
  rotate: "Rotate",
  scale: "Scale",
};

/** Feature ids are new, unique in the captured document and valid for CAD IR. */
export function newFeatureIds(program: string, count: number): string[] | null {
  const document = readTypedCadDocument(program);
  if (!document) return null;
  const taken = new Set(document.features.map((feature) => feature.id));
  const ids: string[] = [];
  while (ids.length < count) {
    const id = `transform_${crypto.randomUUID().replaceAll("-", "").slice(0, 12)}`;
    if (!taken.has(id)) {
      taken.add(id);
      ids.push(id);
    }
  }
  return ids;
}

/**
 * Confirms one staged transform through the host Command API. The host checks
 * the expected revision, permissions and rebuild; this adapter never writes the project.
 */
export async function commitTransform(
  project: Project,
  session: TransformSession,
  current: TransformCapture,
  label: string,
): Promise<
  | { status: "noop" }
  | { status: "committed"; project: Project; last: LastTransform | null }
> {
  const revision = project.revisions.find(
    (item) => item.id === project.currentRevision,
  );
  if (!revision?.program || !project.currentRevision)
    throw new Error("TRANSFORM_STALE");
  const featureIds = newFeatureIds(revision.program, session.targets.length);
  if (!featureIds) throw new Error("TRANSFORM_INPUT_INVALID");
  const name = transformName[session.kind];
  const prepared = prepareTransformCommands(session, current, featureIds, name);
  if (prepared.error) throw new Error(prepared.error);
  if (!prepared.commands.length) return { status: "noop" };
  const next = await invoke<Project>("apply_ir_commands", {
    projectId: project.id,
    expectedRevision: project.currentRevision,
    prompt: label,
    commands: prepared.commands,
  });
  let last: LastTransform | null = null;
  try {
    const head = await lastTransform(next.id);
    if (head && head.definition.expectedRevision === next.currentRevision)
      last = rememberLastTransform(
        session,
        {
          projectId: next.id,
          revisionId: head.definition.expectedRevision,
          sceneToken: `revision:${head.definition.expectedRevision}`,
          transactionId: head.definition.transactionId,
          generation: head.definition.expectedGeneration,
        },
        head.definition.commands.map((command) => command.featureId),
        head.definition.commands.map((command) => command.commandIndex),
        name,
      );
  } catch {
    // The geometry is committed; F9 simply stays unavailable for this operation.
    last = null;
  }
  return { status: "committed", project: next, last };
}
