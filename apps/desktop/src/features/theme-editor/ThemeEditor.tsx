import { useState } from "react";
import { getLocale, t } from "../../i18n";
import { download } from "../../lib/files";
import { useTheme } from "../../lib/theme";
import {
  formatThemeFile, lowContrastPairs, maxThemeFileBytes, parseThemeFile, themeColors,
} from "./themePalette";
import {
  applyThemeTokens, defaultPalettes, useThemeFile, validPalette, type BaseTheme,
} from "./themePreferences";
import "./themeEditor.css";

const labels = {
  background: "App background",
  surface: "Panel surface",
  raisedSurface: "Raised surface",
  text: "Primary text",
  mutedText: "Secondary text",
  border: "Control border",
  accent: "Text accent",
  selection: "Selection color",
  warning: "Warning text",
  measurement: "Measurement color",
  viewport: "Viewport background",
  grid: "Grid color",
  gridMajor: "Major grid color",
  axisX: "X axis color",
  axisY: "Y axis color",
  axisZ: "Z axis color",
} as const;

function ThemePaletteEditor({ theme }: { theme: BaseTheme }) {
  const [source, save] = useThemeFile(theme);
  const [palette, setPalette] = useState(() => validPalette(source) ?? defaultPalettes[theme]);
  const [importFailed, setImportFailed] = useState(false);
  const [status, setStatus] = useState<"applied" | "reset" | null>(null);
  const failures = lowContrastPairs(palette);
  const firstFailure = failures[0];
  const text = formatThemeFile(palette);
  return (
    <details className="theme-editor">
      <summary>{t("Theme editor")}</summary>
      <div className="theme-editor-colors">
        {themeColors.map((key) => (
          <label key={key}>
            {t(labels[key])}
            <input type="color" value={palette[key]} onChange={(event) => {
              setPalette({ ...palette, [key]: event.target.value });
              setStatus(null);
            }} />
          </label>
        ))}
      </div>
      <p className="field-hint" role="status">
        {firstFailure ? t("Low text contrast for {{value0}} on {{value1}} ({{value2}}).", {
          value0: t(labels[firstFailure.foreground]),
          value1: t(labels[firstFailure.background]),
          value2: new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 2 }).format(firstFailure.ratio),
        }) : t("Theme text contrast meets 4.5:1.")}
      </p>
      <div className="theme-editor-actions">
        <button type="button" disabled={!!failures.length} onClick={() => {
          save(text);
          applyThemeTokens(theme);
          setStatus("applied");
        }}>{t("Apply custom theme")}</button>
        <button type="button" onClick={() => {
          save(null);
          setPalette(defaultPalettes[theme]);
          applyThemeTokens(theme);
          setStatus("reset");
        }}>{t("Restore default theme")}</button>
        <button type="button" onClick={() => download(new Blob([text], { type: "application/json" }), "forma-theme.json")}>
          {t("Export theme JSON")}
        </button>
      </div>
      <label>
        {t("Import theme JSON")}
        <input type="file" accept=".json,application/json" onChange={async (event) => {
          const file = event.target.files?.[0];
          event.target.value = "";
          if (!file) return;
          const source = file.size <= maxThemeFileBytes ? await file.text().catch(() => "") : "";
          const imported = parseThemeFile(source);
          setImportFailed(!imported);
          if (imported) {
            setPalette(imported);
            setStatus(null);
          }
        }} />
      </label>
      {importFailed && <p role="status">{t("Invalid theme JSON.")}</p>}
      {status && <p role="status">{status === "applied" ? t("Custom theme applied.") : t("Default theme restored.")}</p>}
    </details>
  );
}

export function ThemeEditor() {
  const theme = useTheme();
  return <ThemePaletteEditor key={theme} theme={theme} />;
}
