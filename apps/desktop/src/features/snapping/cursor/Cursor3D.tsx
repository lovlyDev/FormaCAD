import { useEffect, useMemo } from "react";
import { useThree } from "@react-three/fiber";
import * as THREE from "three";
import { useWorkspace } from "../../../stores/workspace";
import { useTransformViewport } from "../../transform/viewport/transformStore";
import {
  cadPointToRender,
  renderDirectionToCad,
  renderPointToCad,
} from "../../transform/viewport/renderBasis";
import {
  faceForTriangle,
  faceReference,
  faceTriangleCounts,
} from "../../viewer/faceSelection";
import { exactSnap } from "../snapApi";
import { useSnapSettings } from "./snapSettings";

/**
 * 3D cursor used as a pivot. Shift+RMB places it on the surface under the
 * pointer; with exact snapping enabled the tessellation hit is only the ray
 * hint and the point comes from the OCCT face intersection.
 */
export function Cursor3D({
  object,
  light,
}: {
  object: THREE.Group;
  light: boolean;
}) {
  const { camera, gl, invalidate } = useThree();
  const cursorMm = useTransformViewport((state) => state.cursorMm);
  const position = useMemo(() => cadPointToRender(cursorMm), [cursorMm]);
  useEffect(() => {
    const element = gl.domElement;
    const place = (event: PointerEvent) => {
      if (event.button !== 2 || !event.shiftKey) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      const rect = element.getBoundingClientRect();
      const caster = new THREE.Raycaster();
      caster.setFromCamera(
        new THREE.Vector2(
          ((event.clientX - rect.left) / rect.width) * 2 - 1,
          -((event.clientY - rect.top) / rect.height) * 2 + 1,
        ),
        camera,
      );
      const hit = caster
        .intersectObject(object, true)
        .find(
          (item) =>
            item.object instanceof THREE.Mesh &&
            item.object.visible &&
            !item.object.userData.formaSelectionOverlay,
        );
      if (!hit) return;
      const store = useTransformViewport.getState();
      store.setCursor(renderPointToCad(hit.point));
      invalidate();
      const mesh = hit.object as THREE.Mesh;
      const counts = faceTriangleCounts(mesh);
      const ordinal =
        counts && hit.faceIndex != null
          ? faceForTriangle(counts, hit.faceIndex)
          : null;
      const reference = ordinal ? faceReference(mesh, ordinal) : null;
      const project = useWorkspace.getState().project;
      if (
        !useSnapSettings.getState().exact ||
        !reference ||
        !project?.currentRevision
      )
        return;
      const revisionId = project.currentRevision;
      useSnapSettings.getState().setPending(true);
      void exactSnap(
        { projectId: project.id, revisionId, bodyId: mesh.name },
        {
          kind: "face",
          reference,
          rayOriginMm: renderPointToCad(caster.ray.origin),
          rayDirection: renderDirectionToCad(caster.ray.direction),
        },
      )
        .then((result) => {
          // A late answer for an older revision never moves the cursor.
          if (useWorkspace.getState().project?.currentRevision === revisionId) {
            store.setCursor(result.pointMm);
            useSnapSettings.getState().setError(null);
          }
        })
        .catch((cause) => useSnapSettings.getState().setError(cause))
        .finally(() => {
          useSnapSettings.getState().setPending(false);
          invalidate();
        });
    };
    const menu = (event: MouseEvent) => {
      if (event.shiftKey) event.preventDefault();
    };
    element.addEventListener("pointerdown", place, true);
    element.addEventListener("contextmenu", menu, true);
    return () => {
      element.removeEventListener("pointerdown", place, true);
      element.removeEventListener("contextmenu", menu, true);
    };
  }, [camera, gl, object, invalidate]);
  const color = light ? "#c2410c" : "#fb923c";
  return (
    <group position={position} renderOrder={30}>
      <mesh renderOrder={30}>
        <ringGeometry args={[1.4, 2, 32]} />
        <meshBasicMaterial
          color={color}
          depthTest={false}
          side={THREE.DoubleSide}
        />
      </mesh>
      <axesHelper args={[4]} />
    </group>
  );
}
