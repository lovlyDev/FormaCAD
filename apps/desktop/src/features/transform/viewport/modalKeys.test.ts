import { describe, expect, it } from "vitest";
import type { TransformSession } from "../transformSession";
import { modalKey } from "./modalKeys";

const session = (input = "") => ({ input }) as TransformSession;
const key = (
  code: string,
  key: string,
  modifiers: Partial<KeyboardEvent> = {},
) => ({
  code,
  key,
  shiftKey: false,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  ...modifiers,
});

describe("modal transform keys", () => {
  it("uses physical codes for axes under a Russian layout", () => {
    expect(modalKey(session(), key("KeyX", "ч"))).toEqual({
      type: "axis",
      axis: "x",
      exclude: false,
    });
    expect(modalKey(session(), key("KeyZ", "Я", { shiftKey: true }))).toEqual({
      type: "axis",
      axis: "z",
      exclude: true,
    });
  });
  it("types values with units, toggles sign and edits", () => {
    expect(modalKey(session("1"), key("Digit0", "0"))).toEqual({
      type: "input",
      input: "10",
    });
    expect(modalKey(session("10"), key("KeyV", "м"))).toEqual({
      type: "input",
      input: "10м",
    });
    expect(modalKey(session("10"), key("Minus", "-"))).toEqual({
      type: "input",
      input: "-10",
    });
    expect(modalKey(session("-10"), key("Minus", "-"))).toEqual({
      type: "input",
      input: "10",
    });
    expect(modalKey(session("10"), key("Backspace", "Backspace"))).toEqual({
      type: "input",
      input: "1",
    });
  });
  it("switches tool only before typing and swallows global shortcuts", () => {
    expect(modalKey(session(), key("KeyR", "к"))).toEqual({
      type: "kind",
      kind: "rotate",
    });
    expect(modalKey(session("5"), key("KeyR", "r"))).toEqual({
      type: "input",
      input: "5r",
    });
    expect(modalKey(session(), key("KeyZ", "z", { ctrlKey: true }))).toEqual({
      type: "consume",
    });
    expect(modalKey(session(), key("Escape", "Escape"))).toEqual({
      type: "cancel",
    });
    expect(modalKey(session(), key("NumpadEnter", "Enter"))).toEqual({
      type: "confirm",
    });
  });
});
