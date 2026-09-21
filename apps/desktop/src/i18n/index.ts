import { useSyncExternalStore } from "react";
import { persist } from "../lib/persistence";
import en from "./en.json";
import ru from "./ru.json";

export type Locale = "en" | "ru";
const catalogs: Record<Locale, Record<string, string>> = { en, ru };
const listeners = new Set<() => void>();
function initialLocale(): Locale {
  try {
    return localStorage.getItem("forma.locale") === "en" ? "en" : "ru";
  } catch {
    return "ru";
  }
}
let locale = initialLocale();
export const getLocale = () => locale;
export function setLocale(next: Locale) {
  locale = next;
  try {
    persist("forma.locale", next);
  } catch {
    /* Session language still works. */
  }
  document.documentElement.lang = next;
  listeners.forEach((listener) => listener());
}
export function useLocale() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    getLocale,
    () => "ru" as const,
  );
}
// Source-text keys keep the legacy text readable. Both catalogs are mandatory.
// Render system labels with t(); never pass user prompts, source code or filenames to it.
export function t(
  key: string,
  values: Record<string, string | number> = {},
): string {
  const text = catalogs[locale][key] ?? key;
  return text.replace(/\{\{(\w+)\}\}/g, (match, name: string) =>
    String(values[name] ?? match),
  );
}
export const number = (value: number, digits = 2) =>
  new Intl.NumberFormat(locale === "ru" ? "ru-RU" : "en-US", {
    maximumFractionDigits: digits,
  }).format(value);
export const fixedNumber = (value: number, digits = 2) =>
  new Intl.NumberFormat(locale === "ru" ? "ru-RU" : "en-US", {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
    useGrouping: false,
  }).format(value);

export function quantity(kind: "revisions" | "bodies", count: number) {
  const form = new Intl.PluralRules(locale).select(count);
  return t(`${kind}.${form}`, { count });
}

/** Exact catalog/template matching for application-generated historical events. */
export function systemText(text: string): string {
  if (text.startsWith("CAD: ")) return `CAD: ${systemText(text.slice(5))}`;
  if (catalogs[locale][text]) return t(text);
  for (const lang of ["en", "ru"] as const) {
    // Specific event templates must win over broad labels like "Revision {{value0}}".
    const patterns = Object.entries(catalogs[lang]).sort(
      (a, b) =>
        b[1].replace(/\{\{\w+\}\}/g, "").length -
        a[1].replace(/\{\{\w+\}\}/g, "").length,
    );
    for (const [key, pattern] of patterns) {
      if (pattern === text) return t(key);
      if (!pattern.includes("{{")) continue;
      const names: string[] = [];
      const expression = pattern
        .split(/(\{\{\w+\}\})/)
        .map((part) => {
          const name = /^\{\{(\w+)\}\}$/.exec(part);
          if (name) {
            names.push(name[1]);
            return "([\\s\\S]*?)";
          }
          return part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
        })
        .join("");
      const match = new RegExp(`^${expression}$`).exec(text);
      if (match)
        return t(
          key,
          Object.fromEntries(names.map((name, i) => [name, match[i + 1]])),
        );
    }
  }
  return text;
}

export const rawError = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);
export function errorText(error: unknown): string {
  const text = rawError(error);
  try {
    const value: unknown = JSON.parse(text);
    if (
      value &&
      typeof value === "object" &&
      "message" in value &&
      typeof value.message === "string"
    ) {
      const id =
        "featureId" in value && typeof value.featureId === "string"
          ? value.featureId
          : "";
      const code =
        "code" in value && typeof value.code === "string" ? value.code : "";
      return `${[id, code].filter(Boolean).join(" · ")}${id || code ? ": " : ""}${systemText(value.message)}`;
    }
  } catch {
    /* Plain diagnostics remain readable. */
  }
  return systemText(text);
}
