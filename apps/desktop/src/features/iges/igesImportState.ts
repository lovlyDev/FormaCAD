import { create } from "zustand";
import type { Project } from "../../types";

/** In-memory handoff survives the Home → project route remount; never persisted. */
export const useIgesImportState = create<{
  pending: File | null;
  intent: { id: string; project: Project; file: File; awaitingRoute?: boolean } | null;
}>(() => ({ pending: null, intent: null }));
