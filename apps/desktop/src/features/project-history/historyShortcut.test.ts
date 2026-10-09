import { describe, expect, it } from "vitest";
import { historyShortcut } from "./historyShortcut";
function event(key: string, extra: KeyboardEventInit = {}, target?: HTMLElement) {
  const value = new KeyboardEvent("keydown", { key, ctrlKey: true, cancelable: true, ...extra });
  if (target) Object.defineProperty(value, "target", { value: target });
  return value;
}
describe("CAD history keyboard ownership", () => {
  it("recognizes undo, redo, macOS modifier and physical Russian-layout keys", () => {
    expect(historyShortcut(event("z"))).toBe("undo");
    expect(historyShortcut(event("z", { shiftKey: true }))).toBe("redo");
    expect(historyShortcut(event("y"))).toBe("redo");
    expect(historyShortcut(event("я", { code: "KeyZ", ctrlKey: false, metaKey: true }))).toBe("undo");
  });
  it("preserves text, custom editable fields and modal interaction", () => {
    for (const tag of ["input", "textarea", "select"]) expect(historyShortcut(event("z", {}, document.createElement(tag)))).toBeNull();
    const editor = document.createElement("div"); editor.setAttribute("contenteditable", "true"); const child = editor.appendChild(document.createElement("span"));
    expect(historyShortcut(event("z", {}, child))).toBeNull();
    editor.removeAttribute("contenteditable"); editor.setAttribute("role", "dialog");
    expect(historyShortcut(event("y", {}, child))).toBeNull();
  });
  it("ignores repeated, consumed and unrelated key events", () => {
    const consumed = event("z"); consumed.preventDefault(); expect(historyShortcut(consumed)).toBeNull();
    expect(historyShortcut(event("z", { repeat: true }))).toBeNull();
    expect(historyShortcut(event("z", { altKey: true }))).toBeNull();
    expect(historyShortcut(event("z", { ctrlKey: false }))).toBeNull();
    expect(historyShortcut(event("k"))).toBeNull();
  });
});
