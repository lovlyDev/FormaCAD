import { useSyncExternalStore } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { persist } from "./persistence";
import { applyThemeTokens, type BaseTheme } from "../features/theme-editor/themePreferences";

export function applyTheme(value: string) {
  const theme = value === "light" ? "light" : "dark";
  document.documentElement.dataset.theme = theme;
  applyThemeTokens(theme);
  persist("forma.theme", theme);
  if (isTauri()) {
    void getCurrentWindow().setTheme(theme).catch(console.error);
  }
}

const currentTheme = (): BaseTheme =>
  document.documentElement.dataset.theme === "light" ? "light" : "dark";

function subscribe(listener: () => void) {
  const observer = new MutationObserver(listener);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme"],
  });
  return () => observer.disconnect();
}

// React and WebGL follow the same attribute as CSS, without remounting the scene.
export function useTheme() {
  return useSyncExternalStore<BaseTheme>(subscribe, currentTheme, () => "dark");
}
