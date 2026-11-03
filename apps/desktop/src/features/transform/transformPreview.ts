import { Matrix4 } from "three";
import { type TransformFrame, type Vec3, vector } from "./frames";
export type TransformIntent =
  | { kind: "move"; offsetMm: Vec3 }
  | { kind: "rotate"; axis: Vec3; angleDeg: number; pivotMm: Vec3 }
  | { kind: "scale"; factors: Vec3; frame: TransformFrame; pivotMm: Vec3 };
/** Delta in world millimetres. This changes only a preview matrix, never the CAD source. */
export function previewDelta(intent: TransformIntent): Matrix4 {
  if (intent.kind === "move")
    return new Matrix4().makeTranslation(...intent.offsetMm);
  let delta: Matrix4;
  if (intent.kind === "rotate") {
    delta = new Matrix4().makeRotationAxis(
      vector(intent.axis).normalize(),
      (intent.angleDeg * Math.PI) / 180,
    );
  } else {
    const basis = new Matrix4().makeBasis(
      vector(intent.frame.x),
      vector(intent.frame.y),
      vector(intent.frame.z),
    );
    delta = basis
      .clone()
      .multiply(new Matrix4().makeScale(...intent.factors))
      .multiply(basis.clone().invert());
  }
  return new Matrix4()
    .makeTranslation(...intent.pivotMm)
    .multiply(delta)
    .multiply(
      new Matrix4().makeTranslation(
        ...(intent.pivotMm.map((value) => -value) as Vec3),
      ),
    );
}
export function intentIsNoop(intent: TransformIntent): boolean {
  if (intent.kind === "move")
    return intent.offsetMm.every((value) => Math.abs(value) < 1e-12);
  if (intent.kind === "rotate") return Math.abs(intent.angleDeg % 360) < 1e-12;
  return intent.factors.every((value) => Math.abs(value - 1) < 1e-12);
}
export function previewMatrix(
  originalWorld: readonly number[],
  intent: TransformIntent,
): number[] {
  return previewDelta(intent)
    .multiply(new Matrix4().fromArray(originalWorld))
    .toArray();
}
