import { useEffect, useRef } from "react";
import { useThree } from "@react-three/fiber";
import * as THREE from "three";
import {
  cadPointToRender,
  cadWorldFrame,
} from "../transform/viewport/renderBasis";
import { findSnapCandidate } from "../snapping/snapCandidate";
import { resolveSnap } from "../snapping/resolveSnap";

export interface MeasurePoint {
  id: number;
  /** World-space render millimetres; distances and angles are basis-independent. */
  point: THREE.Vector3;
  exact: boolean;
}

/**
 * Ruler/protractor picks: a click snaps to the nearest BREP vertex or the face
 * under the pointer and is then replaced by the OCCT point when available.
 */
export function MeasureSnap({
  object,
  enabled,
  onPick,
  onRefine,
}: {
  object: THREE.Group;
  enabled: boolean;
  onPick: (point: MeasurePoint) => void;
  onRefine: (point: MeasurePoint) => void;
}) {
  const { camera, gl, invalidate } = useThree();
  const next = useRef(1);
  const callbacks = useRef({ onPick, onRefine });
  useEffect(() => {
    callbacks.current = { onPick, onRefine };
  }, [onPick, onRefine]);
  useEffect(() => {
    if (!enabled) return;
    const element = gl.domElement;
    let start: { x: number; y: number } | null = null;
    const down = (event: PointerEvent) => {
      start =
        event.button === 0 ? { x: event.clientX, y: event.clientY } : null;
    };
    const up = (event: PointerEvent) => {
      // A drag orbits the camera; only a click places a point.
      if (
        !start ||
        Math.hypot(event.clientX - start.x, event.clientY - start.y) > 4
      )
        return;
      const rect = element.getBoundingClientRect();
      const frame = cadWorldFrame(object);
      const candidate = findSnapCandidate({
        object,
        camera,
        screen: { x: event.clientX - rect.left, y: event.clientY - rect.top },
        size: { width: rect.width, height: rect.height },
        frame,
      });
      if (!candidate) return;
      const id = next.current++;
      callbacks.current.onPick({
        id,
        point: cadPointToRender(candidate.pointMm, frame),
        exact: false,
      });
      invalidate();
      void resolveSnap(candidate).then((resolved) => {
        if (!resolved?.exact) return;
        callbacks.current.onRefine({
          id,
          point: cadPointToRender(resolved.pointMm, frame),
          exact: true,
        });
        invalidate();
      });
    };
    element.addEventListener("pointerdown", down);
    element.addEventListener("pointerup", up);
    return () => {
      element.removeEventListener("pointerdown", down);
      element.removeEventListener("pointerup", up);
    };
  }, [enabled, camera, gl, object, invalidate]);
  return null;
}
