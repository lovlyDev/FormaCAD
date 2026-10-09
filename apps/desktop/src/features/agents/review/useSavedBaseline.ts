import { useEffect, useState } from "react";
import type { Group } from "three";
import { loadModel } from "../../../lib/files";
import { disposeModel } from "../../../lib/model";
import type { Project } from "../../../types";

/** Own a separate GLB object; never reparent the main workspace scene into review. */
export function useSavedBaseline(base: Project) {
  const [state, setState] = useState<{ object: Group | null; error: boolean; pending: boolean }>({ object: null, error: false, pending: true });
  useEffect(() => {
    let active = true, owned: Group | null = null;
    const name = base.revisions.find(revision => revision.id === base.currentRevision)?.preview;
    const file = base.files.find(file => file.name === name && /\.glb$/i.test(file.name));
    setState({ object: null, error: false, pending: !!file });
    if (file) void loadModel(file, base.id).then(object => {
      if (!active) { disposeModel(object); return; }
      owned = object; setState({ object, error: false, pending: false });
    }, () => { if (active) setState({ object: null, error: true, pending: false }); });
    return () => { active = false; if (owned) disposeModel(owned); };
  }, [base]);
  return state;
}
