import type { HistoryDirection } from "./historyApi";

/** Text editors own their undo stack. CAD shortcuts must never intercept it. */
export function historyShortcut(event: KeyboardEvent): HistoryDirection | null {
  if (event.defaultPrevented || event.repeat || event.altKey || !(event.ctrlKey || event.metaKey)) return null;
  if (event.target instanceof Element && event.target.closest("input,textarea,select,[contenteditable]:not([contenteditable=false]),[role=textbox],[role=combobox],[role=dialog]")) return null;
  const key = event.key.toLowerCase();
  if (event.code === "KeyZ" || key === "z") return event.shiftKey ? "redo" : "undo";
  if ((event.code === "KeyY" || key === "y") && !event.shiftKey) return "redo";
  return null;
}
