import { finiteVector, type Vec3 } from "./frames";
export interface BodyAnchor {
  bodyId: string;
  originMm: Vec3;
  boundsMm: { min: Vec3; max: Vec3 };
}
export type PivotMode =
  "bounds" | "median" | "cursor" | "active" | "individual";
export function validAnchor(anchor: BodyAnchor): boolean {
  return (
    !!anchor.bodyId &&
    finiteVector(anchor.originMm) &&
    finiteVector(anchor.boundsMm.min) &&
    finiteVector(anchor.boundsMm.max) &&
    anchor.boundsMm.min.every((min, index) => min <= anchor.boundsMm.max[index])
  );
}
/** Visual anchors; native execution must recompute/validate authoritative geometry. */
export function selectionPivot(
  anchors: readonly BodyAnchor[],
  mode: PivotMode,
  activeId: string,
  cursorMm: Vec3,
): Vec3 | null {
  if (!anchors.length || !anchors.every(validAnchor)) return null;
  if (mode === "cursor") return finiteVector(cursorMm) ? [...cursorMm] : null;
  if (mode === "active" || mode === "individual") {
    const active = anchors.find((item) => item.bodyId === activeId);
    return active ? [...active.originMm] : null;
  }
  return [0, 1, 2].map((index) =>
    mode === "median"
      ? anchors.reduce((sum, item) => sum + item.originMm[index], 0) /
        anchors.length
      : (Math.min(...anchors.map((item) => item.boundsMm.min[index])) +
          Math.max(...anchors.map((item) => item.boundsMm.max[index]))) /
        2,
  ) as Vec3;
}
