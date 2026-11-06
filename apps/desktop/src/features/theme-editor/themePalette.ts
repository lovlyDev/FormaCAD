import { z } from "zod";

const color = z.string().regex(/^#[0-9a-f]{6}$/i);
const colors = z.strictObject({
  background: color,
  surface: color,
  raisedSurface: color,
  text: color,
  mutedText: color,
  border: color,
  accent: color,
  selection: color,
  warning: color,
  measurement: color,
  viewport: color,
  grid: color,
  gridMajor: color,
  axisX: color,
  axisY: color,
  axisZ: color,
});

const themeFile = z.strictObject({ version: z.literal(1), colors });
export type ThemePalette = z.infer<typeof colors>;
export type ThemeColor = keyof ThemePalette;
export const themeColors = colors.keyof().options;
export const maxThemeFileBytes = 8192;

export function parseThemeFile(source: string): ThemePalette | null {
  if (new TextEncoder().encode(source).length > maxThemeFileBytes) return null;
  try {
    const result = themeFile.safeParse(JSON.parse(source));
    return result.success ? result.data.colors : null;
  } catch {
    return null;
  }
}

export function formatThemeFile(palette: ThemePalette) {
  return JSON.stringify({ version: 1, colors: palette }, null, 2);
}

function luminance(hex: string) {
  const channel = (start: number) => {
    const srgb = Number.parseInt(hex.slice(start, start + 2), 16) / 255;
    return srgb <= 0.04045 ? srgb / 12.92 : ((srgb + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

export function contrastRatio(first: string, second: string) {
  const a = luminance(first);
  const b = luminance(second);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

export function lowContrastPairs(palette: ThemePalette) {
  const foregrounds = ["text", "mutedText", "accent", "warning", "measurement"] as const;
  const surfaces = ["background", "surface", "raisedSurface"] as const;
  return foregrounds.flatMap((foreground) => surfaces.flatMap((background) => {
    const ratio = contrastRatio(palette[foreground], palette[background]);
    return ratio < 4.5 ? [{ foreground, background, ratio }] : [];
  }));
}
