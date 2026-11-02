import {
  useCallback,
  useEffect,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import type * as THREE from "three";
import { useWorkspace } from "../../stores/workspace";
import {
  bodyVisible,
  changeVisibility,
  type VisibilityAction,
} from "./visibilityModel";

/** Outliner keys are `${index}:${name}`; viewport commands work on selected body names. */
const bodyKey = (body: THREE.Object3D, index: number) =>
  `${index}:${body.name}`;

/**
 * Viewport visibility: persistent hiding (shared with the outliner eye) plus a
 * temporary isolation filter. Neither changes BREP, revisions or permissions.
 */
export function useBodyVisibility(
  bodies: readonly THREE.Mesh[],
  hidden: string[],
  setHidden: Dispatch<SetStateAction<string[]>>,
) {
  const [isolated, setIsolated] = useState<string[] | null>(null);
  useEffect(() => {
    // A new scene without the isolated bodies leaves isolation.
    const keys = bodies.map(bodyKey);
    setIsolated((current) =>
      current && !current.some((key) => keys.includes(key)) ? null : current,
    );
  }, [bodies]);
  useEffect(() => {
    const state = { hidden, isolated };
    bodies.forEach((body, index) => {
      body.visible = bodyVisible(state, bodyKey(body, index));
    });
  }, [bodies, hidden, isolated]);
  const onVisibility = useCallback(
    (action: VisibilityAction) => {
      const { selectedBodies, selected } = useWorkspace.getState();
      const names = new Set(
        selectedBodies.length ? selectedBodies : selected ? [selected] : [],
      );
      const all = bodies.map(bodyKey);
      const chosen = bodies.flatMap((body, index) =>
        names.has(body.name) ? [bodyKey(body, index)] : [],
      );
      const next = changeVisibility({ hidden, isolated }, all, chosen, action);
      setHidden(next.hidden);
      setIsolated(next.isolated);
    },
    [bodies, hidden, isolated, setHidden],
  );
  return { isolated, onVisibility };
}
