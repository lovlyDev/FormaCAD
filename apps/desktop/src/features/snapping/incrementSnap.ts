import type { TransformSession } from "../transform/transformSession";
import { snapIncrement } from "./gridSnap";

/** Mathematical increments for the modal tool: they round pointer motion, never claim a BREP point. */
export const incrementSteps = { lengthMm: 1, angleDeg: 5, scale: 0.1 } as const;

export function snapSessionIncrements(
  session: TransformSession,
  enabled: boolean,
  steps: { lengthMm: number; angleDeg: number; scale: number } = incrementSteps,
): TransformSession {
  if (!enabled) return session;
  const { translationMm, angleDeg, scaleDelta } = session.accumulated;
  const translation = translationMm.map((value) =>
    snapIncrement(value, steps.lengthMm),
  );
  const angle = snapIncrement(angleDeg, steps.angleDeg);
  const scale = snapIncrement(1 + scaleDelta, steps.scale);
  if (
    translation.some((value) => value === null) ||
    angle === null ||
    scale === null
  )
    return session;
  return {
    ...session,
    accumulated: {
      translationMm: translation as [number, number, number],
      angleDeg: angle,
      scaleDelta: Math.max(steps.scale, scale) - 1,
    },
  };
}
