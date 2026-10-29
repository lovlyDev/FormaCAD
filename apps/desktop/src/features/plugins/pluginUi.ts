import { create } from "zustand";
import type { InstallPreview, PluginSummary } from "./api";

export type RunKind = "feature" | "command" | "import" | "export";

export interface RunTarget {
  plugin: PluginSummary;
  kind: RunKind;
  contributionId: string;
}

/** Plugin dialogs shared by the settings page, the palette and links. */
interface PluginUi {
  preview: InstallPreview | null;
  run: RunTarget | null;
  panel: { plugin: PluginSummary; panelId: string } | null;
  showPreview: (preview: InstallPreview | null) => void;
  openRun: (target: RunTarget | null) => void;
  openPanel: (panel: { plugin: PluginSummary; panelId: string } | null) => void;
}

export const usePluginUi = create<PluginUi>((set) => ({
  preview: null,
  run: null,
  panel: null,
  showPreview: (preview) => set({ preview }),
  openRun: (run) => set({ run }),
  openPanel: (panel) => set({ panel }),
}));
