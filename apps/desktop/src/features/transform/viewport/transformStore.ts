import { create } from "zustand";
import type { LastTransform } from "../../adjust-last/amendTransform";
import type { PivotMode } from "../pivots";
import type { Vec3 } from "../frames";
import type { TransformSession } from "../transformSession";
import { persist, readState } from "../../../lib/persistence";

const pivotModes: readonly PivotMode[] = [
  "bounds",
  "median",
  "cursor",
  "active",
  "individual",
];

export type TransformStatus =
  | { kind: "idle" }
  | { kind: "committing" }
  | { kind: "error"; message: string };

/** Viewport-owned modal state. CAD writes happen only through the commit route. */
interface TransformViewportState {
  session: TransformSession | null;
  status: TransformStatus;
  last: LastTransform | null;
  adjustOpen: boolean;
  pivotMode: PivotMode;
  incrementSnap: boolean;
  cursorMm: Vec3;
  setSession: (session: TransformSession | null) => void;
  setStatus: (status: TransformStatus) => void;
  setLast: (last: LastTransform | null) => void;
  setAdjustOpen: (open: boolean) => void;
  setPivotMode: (mode: PivotMode) => void;
  setIncrementSnap: (enabled: boolean) => void;
  setCursor: (point: Vec3) => void;
  /** Profile preferences are restored after module load, so read them on viewport mount. */
  hydrate: () => void;
}

export const useTransformViewport = create<TransformViewportState>((set) => ({
  session: null,
  status: { kind: "idle" },
  last: null,
  adjustOpen: false,
  pivotMode: "bounds",
  incrementSnap: false,
  cursorMm: [0, 0, 0],
  setSession: (session) => set({ session }),
  setStatus: (status) => set({ status }),
  setLast: (last) => set({ last }),
  setAdjustOpen: (adjustOpen) => set({ adjustOpen }),
  setPivotMode: (pivotMode) => {
    persist("forma.ui.transform.pivot", JSON.stringify(pivotMode));
    set({ pivotMode });
  },
  setIncrementSnap: (incrementSnap) => {
    persist("forma.ui.transform.increment", JSON.stringify(incrementSnap));
    set({ incrementSnap });
  },
  setCursor: (cursorMm) =>
    cursorMm.every(Number.isFinite)
      ? set({ cursorMm: [...cursorMm] })
      : undefined,
  hydrate: () => {
    const pivot = readState<string>("forma.ui.transform.pivot", "bounds");
    set({
      pivotMode: pivotModes.includes(pivot as PivotMode)
        ? (pivot as PivotMode)
        : "bounds",
      incrementSnap:
        readState<boolean>("forma.ui.transform.increment", false) === true,
    });
  },
}));

export { pivotModes };
