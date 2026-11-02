import * as THREE from "three";
import type { Vec3 } from "../frames";
import type { TransformTarget } from "../transformSession";
import { previewDelta, type TransformIntent } from "../transformPreview";
import { cadDeltaToRender, renderPointToCad } from "./renderBasis";
import { cadVertices } from "../../viewer/vertexSelection";

const identity = new THREE.Matrix4().toArray();

export function bodyMeshes(
  object: THREE.Object3D,
  bodyIds: readonly string[],
): THREE.Mesh[] {
  const wanted = new Set(bodyIds);
  const result: THREE.Mesh[] = [];
  object.traverse((node) => {
    if (
      node instanceof THREE.Mesh &&
      !node.userData.formaSelectionOverlay &&
      wanted.has(node.name)
    )
      result.push(node);
  });
  return result;
}

function vertexBounds(mesh: THREE.Mesh, frame: THREE.Matrix4) {
  const vertices = cadVertices(mesh);
  if (!vertices?.length) return null;
  const min: Vec3 = [Infinity, Infinity, Infinity];
  const max: Vec3 = [-Infinity, -Infinity, -Infinity];
  for (const vertex of vertices) {
    const point = renderPointToCad(
      vertex.point.clone().applyMatrix4(mesh.matrixWorld),
      frame,
    );
    for (const axis of [0, 1, 2]) {
      min[axis] = Math.min(min[axis], point[axis]);
      max[axis] = Math.max(max[axis], point[axis]);
    }
  }
  return { min, max };
}

/** CAD bounds of a rendered body. Transforms are baked into BREP, so the CAD matrix is identity. */
export function transformTarget(
  mesh: THREE.Mesh,
  frame: THREE.Matrix4,
): TransformTarget | null {
  mesh.updateWorldMatrix(true, false);
  const box = new THREE.Box3().setFromObject(mesh, true);
  if (box.isEmpty()) return null;
  const corners = [box.min, box.max].map((point) =>
    renderPointToCad(point, frame),
  );
  const min = [0, 1, 2].map((axis) =>
    Math.min(corners[0][axis], corners[1][axis]),
  ) as Vec3;
  const max = [0, 1, 2].map((axis) =>
    Math.max(corners[0][axis], corners[1][axis]),
  ) as Vec3;
  if (![...min, ...max].every(Number.isFinite)) return null;
  // The tessellation is float32; BREP vertices are written in double precision.
  // When they span the same box (planar bodies), the exact vertex box is the pivot source.
  const exact = vertexBounds(mesh, frame);
  if (
    exact &&
    [0, 1, 2].every(
      (axis) =>
        Math.abs(exact.min[axis] - min[axis]) < 1e-3 &&
        Math.abs(exact.max[axis] - max[axis]) < 1e-3,
    )
  ) {
    min.splice(0, 3, ...exact.min);
    max.splice(0, 3, ...exact.max);
  }
  return {
    bodyId: mesh.name,
    originMm: min.map((value, axis) => (value + max[axis]) / 2) as Vec3,
    boundsMm: { min, max },
    matrixWorld: [...identity],
  };
}

/** Owns original local matrices so cancel and failed commits restore exactly. */
export class PreviewMatrices {
  private originals = new Map<
    THREE.Object3D,
    { matrix: THREE.Matrix4; auto: boolean }
  >();
  constructor(
    private meshes: readonly THREE.Mesh[],
    private frame: THREE.Matrix4,
  ) {
    for (const mesh of meshes) {
      mesh.updateMatrix();
      this.originals.set(mesh, {
        matrix: mesh.matrix.clone(),
        auto: mesh.matrixAutoUpdate,
      });
    }
  }
  apply(intents: readonly { bodyId: string; intent: TransformIntent }[]) {
    const byBody = new Map(intents.map((item) => [item.bodyId, item.intent]));
    for (const mesh of this.meshes) {
      const intent = byBody.get(mesh.name);
      const original = this.originals.get(mesh);
      if (!intent || !original) continue;
      const parentWorld = mesh.parent
        ? mesh.parent.matrixWorld.clone()
        : new THREE.Matrix4();
      const world = parentWorld.clone().multiply(original.matrix);
      // Scale along a rotated frame may shear, so the matrix is set directly instead of decomposed.
      mesh.matrixAutoUpdate = false;
      mesh.matrix.copy(
        parentWorld
          .invert()
          .multiply(cadDeltaToRender(previewDelta(intent), this.frame))
          .multiply(world),
      );
      mesh.updateMatrixWorld(true);
    }
  }
  restore() {
    for (const [mesh, { matrix, auto }] of this.originals) {
      mesh.matrix.copy(matrix);
      matrix.decompose(mesh.position, mesh.quaternion, mesh.scale);
      mesh.matrixAutoUpdate = auto;
      mesh.updateMatrixWorld(true);
    }
  }
}
