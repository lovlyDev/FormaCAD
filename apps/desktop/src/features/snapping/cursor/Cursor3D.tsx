import { useEffect, useMemo } from "react";
import { useThree } from "@react-three/fiber";
import * as THREE from "three";
import { useTransformViewport } from "../../transform/viewport/transformStore";
import {
  cadPointToRender,
  cadWorldFrame,
} from "../../transform/viewport/renderBasis";
import { findSnapCandidate } from "../snapCandidate";
import { resolveSnap } from "../resolveSnap";

/**
 * 3D cursor used as a pivot. Shift+RMB places it on the nearest vertex or the
 * surface under the pointer; with exact snapping enabled the tessellation hit
 * is only a hint and OCCT computes the final point on the committed body.
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
  const position = useMemo(
    () => cadPointToRender(cursorMm, cadWorldFrame(object)),
    [cursorMm, object],
  );
  useEffect(() => {
    const element = gl.domElement;
    const place = (event: PointerEvent) => {
      if (event.button !== 2 || !event.shiftKey) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      const rect = element.getBoundingClientRect();
      const candidate = findSnapCandidate({
        object,
        camera,
        screen: { x: event.clientX - rect.left, y: event.clientY - rect.top },
        size: { width: rect.width, height: rect.height },
        frame: cadWorldFrame(object),
      });
      if (!candidate) return;
      const store = useTransformViewport.getState();
      store.setCursor(candidate.pointMm);
      invalidate();
      void resolveSnap(candidate).then((resolved) => {
        if (resolved) store.setCursor(resolved.pointMm);
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
