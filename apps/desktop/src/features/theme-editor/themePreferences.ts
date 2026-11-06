import { usePersistentState } from "../../lib/persistence";
import { lowContrastPairs, parseThemeFile, type ThemePalette } from "./themePalette";

export type BaseTheme = "dark" | "light";

export const defaultPalettes: Record<BaseTheme, ThemePalette> = {
  dark: {
    background: "#141516", surface: "#1c1e20", raisedSurface: "#25282b",
    text: "#e5e7e9", mutedText: "#a0a6ad", border: "#32363a", accent: "#c4b5fd",
    selection: "#c4b5fd", warning: "#f0b5ae", measurement: "#e5e7e9",
    viewport: "#202326", grid: "#65717e", gridMajor: "#8797a7",
    axisX: "#ce726a", axisY: "#88a780", axisZ: "#7a9bce",
  },
  light: {
    background: "#e9ecf0", surface: "#fafbfc", raisedSurface: "#eef1f5",
    text: "#252b32", mutedText: "#535f6c", border: "#d7dde3", accent: "#6d28d9",
    selection: "#6d28d9", warning: "#922f28", measurement: "#252b32",
    viewport: "#f5f7f9", grid: "#a4b4c2", gridMajor: "#839aaf",
    axisX: "#ce726a", axisY: "#88a780", axisZ: "#7a9bce",
  },
};

const key = (theme: BaseTheme) => `forma.ui.customTheme.${theme}`;

export function useThemeFile(theme: BaseTheme) {
  return usePersistentState<string | null>(key(theme), null);
}

export function validPalette(source: string | null) {
  const palette = source === null ? null : parseThemeFile(source);
  return palette && !lowContrastPairs(palette).length ? palette : null;
}

export function applyThemeTokens(theme: BaseTheme) {
  const stored = localStorage.getItem(key(theme));
  let source: string | null = null;
  try {
    const value: unknown = JSON.parse(stored ?? "null");
    if (typeof value === "string") source = value;
  } catch {
    // Use the base theme for invalid stored preferences.
  }
  const palette = validPalette(source);
  const root = document.documentElement.style;
  const tokens = {
    "--bg": palette?.background,
    "--panel": palette?.surface,
    "--panel2": palette?.raisedSurface,
    "--light-surface": palette?.surface,
    "--text": palette?.text,
    "--light-ink": palette?.text,
    "--muted": palette?.mutedText,
    "--light-secondary": palette?.mutedText,
    "--line": palette?.border,
    "--light-border": palette?.border,
    "--accent-text": palette?.accent,
    "--light-accent-ink": palette?.accent,
    "--light-danger-ink": palette?.warning,
    "--custom-selection": palette?.selection,
    "--custom-measurement": palette?.measurement,
  };
  for (const [property, value] of Object.entries(tokens)) {
    if (value) root.setProperty(property, value);
    else root.removeProperty(property);
  }
}
