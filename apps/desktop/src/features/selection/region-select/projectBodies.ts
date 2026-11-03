import * as THREE from "three";
import type { ProjectedTarget } from "./regionSelection";
import type { Point2, ScreenPrimitive } from "./regionGeometry";

const maxTrianglesPerBody = 20000;

/**
 * Renderer adapter for region selection: body triangles projected with the same
 * camera as the frame. Keys are CAD body ids (mesh names), never triangle numbers.
 */
export function projectBodies(
  object: THREE.Object3D,
  camera: THREE.Camera,
  size: { width: number; height: number },
): ProjectedTarget[] {
  const result: ProjectedTarget[] = [];
  const point = new THREE.Vector3();
  const toScreen = (
    index: number,
    mesh: THREE.Mesh,
    inFront: { value: boolean },
  ): Point2 => {
    point
      .fromBufferAttribute(
        mesh.geometry.getAttribute("position") as THREE.BufferAttribute,
        index,
      )
      .applyMatrix4(mesh.matrixWorld)
      .project(camera);
    if (point.z >= -1 && point.z <= 1) inFront.value = true;
    return [
      ((point.x + 1) / 2) * size.width,
      ((1 - point.y) / 2) * size.height,
    ];
  };
  object.traverse((node) => {
    if (
      !(node instanceof THREE.Mesh) ||
      node.userData.formaSelectionOverlay ||
      !node.name
    )
      return;
    const position = node.geometry.getAttribute("position");
    if (!position) return;
    node.updateWorldMatrix(true, false);
    const index = node.geometry.getIndex();
    const triangles = Math.floor((index ? index.count : position.count) / 3);
    const stride = Math.max(1, Math.ceil(triangles / maxTrianglesPerBody));
    const primitives: ScreenPrimitive[] = [];
    const inFront = { value: false };
    for (let triangle = 0; triangle < triangles; triangle += stride) {
      const vertex = (corner: number) =>
        index ? index.getX(triangle * 3 + corner) : triangle * 3 + corner;
      primitives.push([
        toScreen(vertex(0), node, inFront),
        toScreen(vertex(1), node, inFront),
        toScreen(vertex(2), node, inFront),
      ]);
    }
    result.push({
      target: {
        key: node.name,
        bodyId: node.name,
        kind: "body",
        selectable: !node.userData.formaSelectionLocked,
        visible: node.visible,
      },
      primitives,
      inFrontOfCamera: inFront.value,
      occluded: false,
    });
  });
  return result;
}
