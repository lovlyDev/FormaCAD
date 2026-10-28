import { previewDelta } from "./transformPreview";
import {
  confirmTransform,
  type TransformCapture,
  type TransformSession,
} from "./transformSession";
export interface TransformBodyCommand {
  command: "transform_body";
  bodyId: string;
  featureId: string;
  name: string;
  matrix: number[];
}
/** Convert world-space visual intent to the native row-major 3x4 command contract. */
export function prepareTransformCommands(
  session: TransformSession,
  current: TransformCapture,
  featureIds: readonly string[],
  name: string,
  mode: "create" | "amend" = "create",
): {
  session: TransformSession;
  commands: TransformBodyCommand[];
  error?: "TRANSFORM_STALE" | "TRANSFORM_INPUT_INVALID";
} {
  const confirmation = confirmTransform(session, current, mode);
  if (!confirmation.intents)
    return {
      session: confirmation.session,
      commands: [],
      error: confirmation.error,
    };
  if (
    featureIds.length !== confirmation.intents.length ||
    new Set(featureIds).size !== featureIds.length ||
    !featureIds.every((id) => /^[A-Za-z0-9_-]{1,80}$/.test(id)) ||
    !name.trim() ||
    name.length > 200
  )
    return { session, commands: [], error: "TRANSFORM_INPUT_INVALID" };
  const commands = confirmation.intents.map(
    (item, index): TransformBodyCommand => {
      const elements = previewDelta(item.intent).elements;
      return {
        command: "transform_body",
        bodyId: item.bodyId,
        featureId: featureIds[index],
        name,
        matrix: [
          elements[0],
          elements[4],
          elements[8],
          elements[12],
          elements[1],
          elements[5],
          elements[9],
          elements[13],
          elements[2],
          elements[6],
          elements[10],
          elements[14],
        ],
      };
    },
  );
  if (
    commands.some((item) =>
      item.matrix.some(
        (value, index) =>
          !Number.isFinite(value) ||
          Math.abs(value) > (index % 4 === 3 ? 10000 : 1000),
      ),
    )
  )
    return { session, commands: [], error: "TRANSFORM_INPUT_INVALID" };
  return { session: confirmation.session, commands };
}
