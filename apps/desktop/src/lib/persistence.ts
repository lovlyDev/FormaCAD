import { invoke, isTauri } from "@tauri-apps/api/core";
import { useCallback, useMemo, useSyncExternalStore, type SetStateAction } from "react";

const listeners = new Set<() => void>();
let timer: ReturnType<typeof setTimeout> | undefined;
let pending = Promise.resolve();
export const persistentKey = (key: string) =>
  key === "forma.theme" || key === "forma.locale" || key.startsWith("forma.ui.");

export function preferences() {
  return Object.fromEntries(Object.keys(localStorage).filter(persistentKey).map(key => [key, localStorage.getItem(key)!]));
}

export function flushPreferences() {
  clearTimeout(timer);
  if (!isTauri()) return Promise.resolve();
  const value = preferences();
  pending = pending.catch(() => {}).then(() => invoke("set_ui_preferences", { value }));
  return pending;
}

export function persist(key: string, value: string) {
  localStorage.setItem(key, value);
  listeners.forEach(listener => listener());
  clearTimeout(timer);
  timer = setTimeout(() => void flushPreferences().catch(console.error), 800);
}

export async function restorePreferences() {
  if (!isTauri()) return;
  const stored = await invoke<Record<string, string>>("get_ui_preferences");
  for (const [key, value] of Object.entries(stored)) {
    if (persistentKey(key) && localStorage.getItem(key) === null) localStorage.setItem(key, value);
  }
}

export function readState<T>(key: string, fallback: T): T {
  try { return JSON.parse(localStorage.getItem(key) ?? "null") ?? fallback; }
  catch { return fallback; }
}

export function usePersistentState<T>(key: string, fallback: T) {
  const subscribe = useCallback((listener: () => void) => {
    listeners.add(listener);
    return () => { listeners.delete(listener); };
  }, []);
  const raw = useSyncExternalStore(subscribe, () => localStorage.getItem(key), () => null);
  const value: T = useMemo(() => {
    try { return raw === null ? fallback : JSON.parse(raw); } catch { return fallback; }
  }, [raw, fallback]);
  const set = useCallback((next: SetStateAction<T>) => {
    const current = readState(key, fallback);
    persist(key, JSON.stringify(typeof next === "function" ? (next as (v: T) => T)(current) : next));
  }, [key, fallback]);
  return [value, set] as const;
}
