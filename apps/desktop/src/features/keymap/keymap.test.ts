import { describe, expect, it } from "vitest";
import { bindingConflicts, defaultBindings, readKeymap } from "./registry";
import { ownsTextInput, resolveKeymap } from "./resolveKeymap";
const available = new Set(defaultBindings.map((item) => item.commandId));
const resolve = (code: string, extra: KeyboardEventInit = {}, options = {}) =>
  resolveKeymap(
    new KeyboardEvent("keydown", { code, ...extra }),
    defaultBindings,
    { context: "viewport", available, ...options },
  );
describe("physical key routing and ownership", () => {
  it("works on Russian layout without matching translated key letters", () => {
    expect(resolve("KeyG", { key: "п" })).toBe("transform.move");
    expect(resolve("KeyZ", { key: "я", metaKey: true })).toBe("history.undo");
    expect(resolve("KeyZ", { ctrlKey: true, shiftKey: true })).toBe(
      "history.redo",
    );
    expect(resolve("KeyG", { shiftKey: true })).toBeNull();
  });
  it("resolves numpad emulation before selection digits and respects modal priority", () => {
    expect(resolve("Digit1")).toBe("selection.body");
    expect(resolve("Digit1", {}, { emulateNumpad: true })).toBe("view.front");
    expect(resolve("Digit1", { ctrlKey: true }, { emulateNumpad: true })).toBe(
      "view.back",
    );
    expect(resolve("KeyZ", {}, { context: "modal:transform" })).toBe(
      "transform.axisZ",
    );
    expect(
      resolve("KeyZ", { ctrlKey: true }, { context: "modal:transform" }),
    ).toBeNull();
  });
  it("does not steal editing, IME, held keys or unavailable actions", () => {
    for (const extra of [{ repeat: true }, { isComposing: true }])
      expect(resolve("KeyG", extra)).toBeNull();
    expect(resolve("KeyG", {}, { textInput: true })).toBeNull();
    expect(resolve("KeyG", {}, { available: new Set<string>() })).toBeNull();
    const input = document.createElement("div");
    input.contentEditable = "true";
    input.setAttribute("contenteditable", "true");
    expect(
      ownsTextInput(input.appendChild(document.createElement("span"))),
    ).toBe(true);
    expect(ownsTextInput(document.createElement("canvas"))).toBe(false);
    const consumed = new KeyboardEvent("keydown", {
      code: "KeyG",
      cancelable: true,
    });
    consumed.preventDefault();
    expect(
      resolveKeymap(consumed, defaultBindings, {
        context: "viewport",
        available,
      }),
    ).toBeNull();
  });
  it("rejects conflicting or unbounded remaps before using the profile", () => {
    expect(bindingConflicts(defaultBindings)).toEqual([]);
    expect(readKeymap(defaultBindings)).toHaveLength(defaultBindings.length);
    expect(readKeymap([...defaultBindings, defaultBindings[0]])).toBeNull();
    expect(
      readKeymap([
        { commandId: "arbitrary", context: "global", chord: { code: "KeyG" } },
      ]),
    ).toBeNull();
    expect(
      readKeymap([
        { ...defaultBindings[0], chord: { code: "F3", primary: "false" } },
      ]),
    ).toBeNull();
  });
});
