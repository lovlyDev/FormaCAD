import { describe, expect, it } from "vitest";
import { contrastRatio, formatThemeFile, lowContrastPairs, parseThemeFile } from "./themePalette";
import { defaultPalettes } from "./themePreferences";

describe("custom themes", () => {
  it("round-trips each base theme through the versioned JSON format", () => {
    for (const palette of Object.values(defaultPalettes)) {
      expect(parseThemeFile(formatThemeFile(palette))).toEqual(palette);
      expect(lowContrastPairs(palette)).toEqual([]);
    }
  });

  it("rejects oversized, partial and invalid colors", () => {
    expect(parseThemeFile(" ".repeat(8193))).toBeNull();
    expect(parseThemeFile('{"version":1,"colors":{}}')).toBeNull();
    expect(parseThemeFile(formatThemeFile({ ...defaultPalettes.dark, text: "red" }))).toBeNull();
  });

  it("reports insufficient sRGB text contrast", () => {
    expect(contrastRatio("#ffffff", "#000000")).toBe(21);
    expect(contrastRatio("#777777", "#ffffff")).toBeLessThan(4.5);
    expect(lowContrastPairs({ ...defaultPalettes.dark, text: defaultPalettes.dark.surface }))
      .toContainEqual({ foreground: "text", background: "surface", ratio: 1 });
  });
});
