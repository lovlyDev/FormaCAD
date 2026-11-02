import { Matrix4, Vector3 } from "three";
import { frameFromMatrix, worldFrame, type TransformFrame } from "./frames";
import type { TypedCadDocument } from "../../lib/typedCadDocument";

type Feature = TypedCadDocument["features"][number];

function linearPart(feature: Feature): Matrix4 | null {
  const op = feature.operation as Record<string, unknown> & { type: string };
  if (op.type === "translate") return new Matrix4();
  if (
    op.type === "transform" &&
    Array.isArray(op.matrix) &&
    op.matrix.length === 12
  ) {
    const m = op.matrix as number[];
    return new Matrix4().set(
      m[0],
      m[1],
      m[2],
      0,
      m[4],
      m[5],
      m[6],
      0,
      m[8],
      m[9],
      m[10],
      0,
      0,
      0,
      0,
      1,
    );
  }
  if (
    op.type === "rotate" &&
    Array.isArray(op.axisDirection) &&
    typeof op.angleDeg === "number"
  ) {
    const axis = new Vector3(...(op.axisDirection as [number, number, number]));
    if (axis.lengthSq() < 1e-18) return null;
    return new Matrix4().makeRotationAxis(
      axis.normalize(),
      (op.angleDeg * Math.PI) / 180,
    );
  }
  if (op.type === "mirror" && Array.isArray(op.planeNormal)) {
    const n = new Vector3(...(op.planeNormal as [number, number, number]));
    if (n.lengthSq() < 1e-18) return null;
    n.normalize();
    // Householder reflection I − 2nnᵀ.
    // prettier-ignore
    return new Matrix4().set(
      1 - 2 * n.x * n.x, -2 * n.x * n.y, -2 * n.x * n.z, 0,
      -2 * n.y * n.x, 1 - 2 * n.y * n.y, -2 * n.y * n.z, 0,
      -2 * n.z * n.x, -2 * n.z * n.y, 1 - 2 * n.z * n.z, 0,
      0, 0, 0, 1,
    );
  }
  return null;
}

/**
 * The body's local frame: rigid/affine transform features applied on top of
 * the construction are baked into the BREP, so their accumulated linear part
 * is the only record of how the body is oriented. Construction features
 * (extrude, boolean, fillet, …) start the chain in the world frame. The result
 * is orthonormalized and right-handed; scale and shear never enter the axes.
 */
export function bodyLocalFrame(
  document: TypedCadDocument,
  bodyId: string,
): TransformFrame {
  const body = document.bodies.find((item) => item.id === bodyId);
  if (!body) return worldFrame;
  const byId = new Map(
    document.features.map((feature) => [feature.id, feature]),
  );
  const chain: Matrix4[] = [];
  let current = byId.get(body.sourceFeatureId);
  const seen = new Set<string>();
  while (current && !seen.has(current.id) && chain.length < 256) {
    seen.add(current.id);
    if (current.suppressed) {
      const source = (current.operation as { bodyFeatureId?: unknown })
        .bodyFeatureId;
      current = typeof source === "string" ? byId.get(source) : undefined;
      continue;
    }
    const linear = linearPart(current);
    if (!linear) break;
    chain.push(linear);
    const source = (current.operation as { bodyFeatureId?: unknown })
      .bodyFeatureId;
    current = typeof source === "string" ? byId.get(source) : undefined;
  }
  // The chain was collected newest first; the newest transform is applied last.
  const total = chain.reduce(
    (acc, linear) => acc.multiply(linear),
    new Matrix4(),
  );
  return frameFromMatrix(total.toArray()) ?? worldFrame;
}
