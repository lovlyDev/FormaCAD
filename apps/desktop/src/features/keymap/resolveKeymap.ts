import type { KeyBinding, KeyContext } from "./registry";
interface KeyEvent {
  code: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  defaultPrevented: boolean;
  repeat: boolean;
  isComposing: boolean;
  getModifierState?: (name: string) => boolean;
}
export function ownsTextInput(target: EventTarget | null): boolean {
  return (
    target instanceof Element &&
    !!target.closest(
      "input,textarea,select,[contenteditable]:not([contenteditable=false]),[role=textbox],[role=combobox],[role=dialog]",
    )
  );
}
/** No listeners or side effects here. Modal ownership excludes all global shortcuts. */
export function resolveKeymap(
  event: KeyEvent,
  bindings: readonly KeyBinding[],
  options: {
    context: KeyContext;
    textInput?: boolean;
    emulateNumpad?: boolean;
    available: ReadonlySet<string>;
  },
): string | null {
  if (
    event.defaultPrevented ||
    event.repeat ||
    event.isComposing ||
    options.textInput ||
    event.getModifierState?.("AltGraph")
  )
    return null;
  let code = event.code;
  if (options.emulateNumpad && options.context === "viewport") {
    if (/^Digit[0-9]$/.test(code)) code = code.replace("Digit", "Numpad");
    if (code === "Period") code = "NumpadDecimal";
    if (code === "Slash") code = "NumpadDivide";
  }
  const contexts: KeyContext[] =
    options.context === "modal:transform"
      ? [options.context]
      : [options.context, "global"];
  for (const context of contexts) {
    const match = bindings.find(
      (item) =>
        item.context === context &&
        options.available.has(item.commandId) &&
        item.chord.code === code &&
        !!item.chord.primary === (event.ctrlKey || event.metaKey) &&
        !!item.chord.shift === event.shiftKey &&
        !!item.chord.alt === event.altKey,
    );
    if (match) return match.commandId;
  }
  return null;
}
