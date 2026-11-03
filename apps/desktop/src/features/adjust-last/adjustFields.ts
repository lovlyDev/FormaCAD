import type { Axis, Vec3 } from "../transform/frames";
import {
  transformIntents,
  type TransformSession,
} from "../transform/transformSession";

/** Editable parameters of the original operation, independent of how it was entered. */
export type AdjustFields =
  | { kind: "move"; offsetMm: Vec3 }
  | { kind: "rotate"; axis: Axis; angleDeg: number }
  | { kind: "scale"; axis: Axis | null; factor: number };

export function fieldsFromSession(
  session: TransformSession,
): AdjustFields | null {
  const intent = transformIntents(session)?.[0]?.intent;
  if (!intent) return null;
  if (intent.kind === "move")
    return { kind: "move", offsetMm: [...intent.offsetMm] };
  const axis = session.constraint?.axis ?? "z";
  if (intent.kind === "rotate")
    return { kind: "rotate", axis, angleDeg: intent.angleDeg };
  const factor =
    intent.factors.find((value) => Math.abs(value - 1) > 1e-12) ?? 1;
  return {
    kind: "scale",
    axis: session.constraint && !session.constraint.exclude ? axis : null,
    factor,
  };
}

/** Rebuilds the session from the original snapshot, so 10 → 20 mm replaces instead of stacking. */
export function sessionFromFields(
  base: TransformSession,
  fields: AdjustFields,
): TransformSession {
  const constraint =
    fields.kind === "move" || (fields.kind === "scale" && fields.axis === null)
      ? null
      : {
          axis: (fields.kind === "rotate" ? fields.axis : fields.axis) as Axis,
          exclude: false,
          space: "orientation" as const,
        };
  return {
    ...base,
    kind: fields.kind,
    phase: "preview",
    input: "",
    constraint,
    sample: { translationMm: [0, 0, 0], angleDeg: 0, scaleDelta: 0 },
    accumulated: {
      translationMm: fields.kind === "move" ? [...fields.offsetMm] : [0, 0, 0],
      angleDeg: fields.kind === "rotate" ? fields.angleDeg : 0,
      scaleDelta: fields.kind === "scale" ? fields.factor - 1 : 0,
    },
  };
}
