import { create } from "zustand";
import { persist, readState } from "../../../lib/persistence";

interface SnapSettings {
  exact: boolean;
  pending: boolean;
  error: unknown;
  setExact: (exact: boolean) => void;
  setPending: (pending: boolean) => void;
  setError: (error: unknown) => void;
  hydrate: () => void;
}

/** Exact snapping waits for the isolated worker, so its progress and failures are visible. */
export const useSnapSettings = create<SnapSettings>((set) => ({
  exact: true,
  pending: false,
  error: null,
  setExact: (exact) => {
    persist("forma.ui.snap.exact", JSON.stringify(exact));
    set({ exact });
  },
  setPending: (pending) => set({ pending }),
  setError: (error) => set({ error }),
  hydrate: () =>
    set({ exact: readState<boolean>("forma.ui.snap.exact", true) !== false }),
}));
