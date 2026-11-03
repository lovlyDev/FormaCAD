import type { Axis } from "../frames";
import type { TransformSession } from "../transformSession";

export type ModalKeyAction =
  | { type: "confirm" }
  | { type: "cancel" }
  | { type: "axis"; axis: Axis; exclude: boolean }
  | { type: "kind"; kind: TransformSession["kind"] }
  | { type: "input"; input: string }
  | { type: "consume" };

const axisCodes: Record<string, Axis> = { KeyX: "x", KeyY: "y", KeyZ: "z" };
const kindCodes: Record<string, TransformSession["kind"]> = {
  KeyG: "move",
  KeyR: "rotate",
  KeyS: "scale",
};
const numericCharacter = /^[0-9.,+\-a-zа-яё°%"]$/iu;

/**
 * Modal ownership: every key is consumed while the tool is active so global
 * shortcuts (including undo) cannot run mid-transform. Commands use physical
 * codes, so axes work under a Russian layout; typed values use the character.
 */
export function modalKey(
  session: TransformSession,
  event: Pick<
    KeyboardEvent,
    "code" | "key" | "shiftKey" | "ctrlKey" | "metaKey" | "altKey"
  >,
): ModalKeyAction {
  if (event.code === "Escape") return { type: "cancel" };
  if (
    event.code === "Enter" ||
    event.code === "NumpadEnter" ||
    event.code === "Space"
  )
    return { type: "confirm" };
  if (event.ctrlKey || event.metaKey || event.altKey)
    return { type: "consume" };
  if (axisCodes[event.code])
    return {
      type: "axis",
      axis: axisCodes[event.code],
      exclude: event.shiftKey,
    };
  if (!session.input && kindCodes[event.code])
    return { type: "kind", kind: kindCodes[event.code] };
  if (event.code === "Backspace")
    return { type: "input", input: session.input.slice(0, -1) };
  if (event.code === "Minus" || event.code === "NumpadSubtract")
    return {
      type: "input",
      input: session.input.startsWith("-")
        ? session.input.slice(1)
        : `-${session.input}`,
    };
  const numpad = /^Numpad([0-9])$/.exec(event.code);
  if (numpad) return { type: "input", input: session.input + numpad[1] };
  if (event.code === "NumpadDecimal")
    return { type: "input", input: `${session.input}.` };
  if (
    event.key.length === 1 &&
    numericCharacter.test(event.key) &&
    session.input.length < 48
  )
    return { type: "input", input: session.input + event.key };
  return { type: "consume" };
}
