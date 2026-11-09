import { Vector3 } from "three";
import { readTypedCadDocument } from "../../../lib/typedCadDocument";
import { readDatumOperation, type AbsoluteDatum, type DatumOperation } from "./datumModel";

type RelativeDatum = Extract<DatumOperation, { type: "datumOnReference" }>;

function relativeToPlane(datum: RelativeDatum, plane: Extract<AbsoluteDatum, { type: "datumPlane" }>): AbsoluteDatum | null {
  const normal = new Vector3(...plane.normal).normalize();
  const tangent = Math.abs(normal.x) < 0.9 ? new Vector3(1, 0, 0) : new Vector3(0, 1, 0);
  tangent.addScaledVector(normal, -tangent.dot(normal)).normalize();
  const angle = datum.anchor.angleDeg * Math.PI / 180;
  const originMm = new Vector3(...plane.originMm).addScaledVector(normal, datum.anchor.offsetMm).toArray();
  const direction = normal.multiplyScalar(Math.cos(angle)).addScaledVector(tangent, Math.sin(angle)).toArray();
  const geometry = datum.geometry;
  const resolved = readDatumOperation(geometry.type === "plane"
    ? { type: "datumPlane", originMm, normal: direction, sideMm: geometry.sideMm }
    : geometry.type === "axis"
      ? { type: "datumAxis", originMm, direction, lengthMm: geometry.lengthMm }
      : { type: "datumPoint", originMm });
  return resolved && resolved.type !== "datumOnReference" ? resolved : null;
}

export function projectDatums(program: string | null | undefined, nativeFrames: unknown) {
  const document = program && readTypedCadDocument(program);
  if (!document) return [];
  const frames = new Map<string, AbsoluteDatum>();
  for (const feature of document.features) {
    const datum = readDatumOperation(feature.operation);
    if (!datum || feature.suppressed) continue;
    if (datum.type !== "datumOnReference") {
      frames.set(feature.id, datum);
      continue;
    }
    if (nativeFrames && typeof nativeFrames === "object" && Object.prototype.hasOwnProperty.call(nativeFrames, feature.id)) {
      const resolved = readDatumOperation((nativeFrames as Record<string, unknown>)[feature.id]);
      if (resolved && resolved.type !== "datumOnReference") frames.set(feature.id, resolved);
    } else if (datum.anchor.source.type === "plane") {
      const plane = frames.get(datum.anchor.source.featureId);
      const resolved = plane?.type === "datumPlane" ? relativeToPlane(datum, plane) : null;
      if (resolved) frames.set(feature.id, resolved);
    }
  }
  return Array.from(frames, ([id, datum]) => ({ id, datum }));
}
