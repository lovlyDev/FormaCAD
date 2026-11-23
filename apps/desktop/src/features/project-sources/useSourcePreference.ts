import { useCallback, useRef, useState, type SetStateAction } from "react";
import { usePersistentState } from "../../lib/persistence";
import type { ProjectSourceBinding } from "./sourceBinding";

/** Folder drafts stay in memory; process-local source IDs never enter preferences. */
export function useSourcePreference<T>(key: string, fallback: T, binding: ProjectSourceBinding | null) {
  const folder = binding?.kind === "folder";
  const [persistent, savePersistent] = usePersistentState(key, fallback, !folder);
  const family = JSON.stringify([binding?.openingId, key]);
  const latest = useRef(family); latest.current = family;
  const [memory, setMemory] = useState({ family, value: fallback });
  const value = folder ? memory.family === family ? memory.value : fallback : persistent;
  const save = useCallback((next: SetStateAction<T>) => {
    if (latest.current !== family) return;
    if (!folder) { savePersistent(next); return; }
    setMemory(previous => {
      if (latest.current !== family) return previous;
      const current = previous.family === family ? previous.value : fallback;
      return { family, value: typeof next === "function" ? (next as (value: T) => T)(current) : next };
    });
  }, [family, folder, fallback, savePersistent]);
  return [value, save] as const;
}
