import type { Vec3 } from "../transform/frames";
import type { TransformSession } from "../transform/transformSession";

/**
 * Moves the pivot onto a snap target: translation = target − pivot. Only free
 * pointer moves snap; a typed value always wins. Axis constraints still apply
 * afterwards in `transformIntents`.
 */
export function applyVertexSnap(
  session: TransformSession,
  targetMm: Vec3 | null,
): TransformSession {
  if (!targetMm || session.kind !== "move" || session.input) return session;
  return {
    ...session,
    accumulated: {
      ...session.accumulated,
      translationMm: [0, 1, 2].map(
        (axis) => targetMm[axis] - session.pivotMm[axis],
      ) as Vec3,
    },
  };
}
