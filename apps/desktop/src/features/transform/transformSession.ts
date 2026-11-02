import {
  type Axis,
  type TransformFrame,
  type Vec3,
  finiteVector,
  validFrame,
  validWorldMatrix,
  vector,
  tuple,
} from "./frames";
import {
  cycleConstraint,
  constrainVector,
  constrainedDirection,
  type AxisConstraint,
} from "./constraints";
import { parseTransformNumber } from "./numericInput";
import {
  selectionPivot,
  validAnchor,
  type BodyAnchor,
  type PivotMode,
} from "./pivots";
import {
  intentIsNoop,
  previewMatrix,
  type TransformIntent,
} from "./transformPreview";
export interface TransformCapture {
  projectId: string;
  revisionId: string;
  sceneToken: string;
}
export interface TransformTarget extends BodyAnchor {
  /** Canonical CAD-mm world matrix; renderer adapters must convert their display basis first. */
  matrixWorld: number[];
}
export interface PointerSample {
  translationMm: Vec3;
  angleDeg: number;
  scaleDelta: number;
}
export interface TransformSession {
  capture: TransformCapture;
  phase: "preview" | "committing" | "cancelled" | "completed";
  kind: TransformIntent["kind"];
  targets: TransformTarget[];
  pivotMode: PivotMode;
  pivotMm: Vec3;
  orientation: TransformFrame;
  local: TransformFrame;
  constraint: AxisConstraint | null;
  input: string;
  sample: PointerSample;
  accumulated: PointerSample;
}
const zeroSample = (): PointerSample => ({
  translationMm: [0, 0, 0],
  angleDeg: 0,
  scaleDelta: 0,
});
export function beginTransform(options: {
  capture: TransformCapture;
  kind: TransformIntent["kind"];
  targets: readonly TransformTarget[];
  activeId: string;
  pivotMode: PivotMode;
  cursorMm: Vec3;
  orientation: TransformFrame;
  local: TransformFrame;
}): TransformSession | null {
  if (
    !options.capture.projectId ||
    !options.capture.revisionId ||
    !options.capture.sceneToken ||
    !validFrame(options.orientation) ||
    !validFrame(options.local)
  )
    return null;
  if (
    !options.targets.length ||
    options.targets.length > 256 ||
    new Set(options.targets.map((item) => item.bodyId)).size !==
      options.targets.length
  )
    return null;
  if (
    !options.targets.every(
      (item) => validAnchor(item) && validWorldMatrix(item.matrixWorld),
    )
  )
    return null;
  const pivotMm = selectionPivot(
    options.targets,
    options.pivotMode,
    options.activeId,
    options.cursorMm,
  );
  if (!pivotMm) return null;
  return {
    capture: { ...options.capture },
    kind: options.kind,
    phase: "preview",
    targets: structuredClone([...options.targets]),
    pivotMode: options.pivotMode,
    pivotMm,
    orientation: structuredClone(options.orientation),
    local: structuredClone(options.local),
    constraint: null,
    input: "",
    sample: zeroSample(),
    accumulated: zeroSample(),
  };
}
export function setTransformInput(
  session: TransformSession,
  input: string,
): TransformSession {
  return session.phase === "preview" && input.length <= 48
    ? { ...session, input }
    : session;
}
export function setTransformAxis(
  session: TransformSession,
  axis: Axis,
  exclude = false,
): TransformSession {
  return session.phase === "preview"
    ? {
        ...session,
        constraint: cycleConstraint(session.constraint, axis, exclude),
      }
    : session;
}
/** Integrating pointer segments avoids a jump when Shift is toggled halfway through a drag. */
export function moveTransformPointer(
  session: TransformSession,
  sample: PointerSample,
  precise: boolean,
): TransformSession {
  if (
    session.phase !== "preview" ||
    !finiteVector(sample.translationMm) ||
    !Number.isFinite(sample.angleDeg) ||
    !Number.isFinite(sample.scaleDelta)
  )
    return session;
  const sensitivity = precise ? 0.1 : 1;
  return {
    ...session,
    sample: structuredClone(sample),
    accumulated: {
      translationMm: tuple(
        vector(session.accumulated.translationMm).add(
          vector(sample.translationMm)
            .sub(vector(session.sample.translationMm))
            .multiplyScalar(sensitivity),
        ),
      ),
      angleDeg:
        session.accumulated.angleDeg +
        (sample.angleDeg - session.sample.angleDeg) * sensitivity,
      scaleDelta:
        session.accumulated.scaleDelta +
        (sample.scaleDelta - session.sample.scaleDelta) * sensitivity,
    },
  };
}
export function transformIntents(
  session: TransformSession,
): { bodyId: string; intent: TransformIntent; matrixWorld: number[] }[] | null {
  const quantity =
    session.kind === "move"
      ? "length"
      : session.kind === "rotate"
        ? "angle"
        : "scale";
  const input = parseTransformNumber(session.input, quantity);
  if (input.status === "invalid" || input.status === "incomplete") return null;
  const numeric = input.status === "valid" ? input.value : null;
  const { orientation, local, constraint } = session;
  const frame = constraint?.space === "local" ? local : orientation;
  const result = session.targets.map((target) => {
    const pivotMm =
      session.pivotMode === "individual" ? target.originMm : session.pivotMm;
    let intent: TransformIntent;
    if (session.kind === "move") {
      const offsetMm =
        numeric === null
          ? constrainVector(
              session.accumulated.translationMm,
              constraint,
              orientation,
              local,
            )
          : tuple(
              vector(
                constrainedDirection(
                  constraint,
                  orientation,
                  local,
                  session.accumulated.translationMm,
                ),
              ).multiplyScalar(numeric),
            );
      intent = { kind: "move", offsetMm };
    } else if (session.kind === "rotate") {
      intent = {
        kind: "rotate",
        axis: [...frame[constraint?.axis ?? "z"]],
        angleDeg: numeric ?? session.accumulated.angleDeg,
        pivotMm: [...pivotMm],
      };
    } else {
      const factor = numeric ?? 1 + session.accumulated.scaleDelta;
      const factors = (["x", "y", "z"] as const).map((axis) =>
        !constraint ||
        (constraint.exclude
          ? axis !== constraint.axis
          : axis === constraint.axis)
          ? factor
          : 1,
      ) as Vec3;
      intent = {
        kind: "scale",
        factors,
        frame: structuredClone(frame),
        pivotMm: [...pivotMm],
      };
    }
    return {
      bodyId: target.bodyId,
      intent,
      matrixWorld: previewMatrix(target.matrixWorld, intent),
    };
  });
  return result.every(
    (item) =>
      item.matrixWorld.every(Number.isFinite) &&
      (item.intent.kind !== "move" ||
        item.intent.offsetMm.every((value) => Math.abs(value) <= 10000)) &&
      (item.intent.kind !== "rotate" ||
        Math.abs(item.intent.angleDeg) <= 360) &&
      (item.intent.kind !== "scale" ||
        item.intent.factors.every((value) => value >= 1e-6 && value <= 1000)),
  )
    ? result
    : null;
}
export function captureMatches(
  expected: TransformCapture,
  current: TransformCapture,
): boolean {
  return (
    expected.projectId === current.projectId &&
    expected.revisionId === current.revisionId &&
    expected.sceneToken === current.sceneToken
  );
}
export function confirmTransform(
  session: TransformSession,
  current: TransformCapture,
  mode: "create" | "amend" = "create",
): {
  session: TransformSession;
  intents: ReturnType<typeof transformIntents>;
  error?: "TRANSFORM_STALE" | "TRANSFORM_INPUT_INVALID";
} {
  if (session.phase !== "preview") return { session, intents: null };
  if (!captureMatches(session.capture, current))
    return {
      session: { ...session, phase: "cancelled" },
      intents: null,
      error: "TRANSFORM_STALE",
    };
  const intents = transformIntents(session);
  if (!intents)
    return { session, intents: null, error: "TRANSFORM_INPUT_INVALID" };
  if (mode === "create" && intents.every((item) => intentIsNoop(item.intent)))
    return { session: { ...session, phase: "cancelled" }, intents: null };
  return { session: { ...session, phase: "committing" }, intents };
}
export function cancelTransform(session: TransformSession): TransformSession {
  return session.phase === "preview"
    ? { ...session, phase: "cancelled" }
    : session;
}
