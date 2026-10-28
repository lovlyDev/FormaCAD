import {
  Box,
  Camera,
  Clapperboard,
  Cog,
  Compass,
  Film,
  Grid3X3,
  Layers,
  MonitorPlay,
  Package,
  PenTool,
  Printer,
  Rotate3d,
  Ruler,
  Scan,
  Scissors,
  Sparkles,
  SquareDashed,
  Video,
  Wand2,
  Wrench,
  type LucideIcon,
} from "lucide-react";

/**
 * Lucide names a plugin may give its feature or command (`icon` in the
 * manifest) to get its own 3D-view toolbar button. A fixed set keeps the
 * bundle small; any other name keeps the action in the "Plugin actions" menu.
 */
const ICONS: Record<string, LucideIcon> = {
  box: Box,
  camera: Camera,
  clapperboard: Clapperboard,
  cog: Cog,
  compass: Compass,
  film: Film,
  "grid-3x3": Grid3X3,
  layers: Layers,
  "monitor-play": MonitorPlay,
  package: Package,
  "pen-tool": PenTool,
  printer: Printer,
  "rotate-3d": Rotate3d,
  ruler: Ruler,
  scan: Scan,
  scissors: Scissors,
  sparkles: Sparkles,
  "square-dashed": SquareDashed,
  video: Video,
  "wand-2": Wand2,
  wrench: Wrench,
};

export const pluginIcon = (name: string | undefined): LucideIcon | null => (name ? (ICONS[name] ?? null) : null);
