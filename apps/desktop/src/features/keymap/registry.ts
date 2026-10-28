export type KeyContext = "global" | "viewport" | "outliner" | "modal:transform";
export type KeyPreset = "forma" | "blender";
export interface KeyChord {
  code: string;
  primary?: boolean;
  shift?: boolean;
  alt?: boolean;
}
export interface KeyBinding {
  commandId: string;
  context: KeyContext;
  chord: KeyChord;
}
const binding = (
  commandId: string,
  code: string,
  modifiers: Omit<KeyChord, "code"> = {},
  context: KeyContext = "viewport",
): KeyBinding => ({ commandId, context, chord: { code, ...modifiers } });

/** Independently authored Forma commands; only available commands are dispatched. */
export const defaultBindings: readonly KeyBinding[] = [
  binding("command.search", "F3", {}, "global"),
  binding("command.search", "KeyK", { primary: true }, "global"),
  binding("history.undo", "KeyZ", { primary: true }, "global"),
  binding("history.redo", "KeyZ", { primary: true, shift: true }, "global"),
  binding("transform.move", "KeyG"),
  binding("transform.rotate", "KeyR"),
  binding("transform.scale", "KeyS"),
  binding("transform.adjustLast", "F9"),
  binding("selection.all", "KeyA"),
  binding("selection.none", "KeyA", { alt: true }),
  binding("selection.invert", "KeyI", { primary: true }),
  binding("selection.box", "KeyB"),
  binding("selection.circle", "KeyC"),
  ...["body", "face", "edge", "vertex"].map((kind, index) =>
    binding(`selection.${kind}`, `Digit${index + 1}`),
  ),
  binding("visibility.hide", "KeyH"),
  binding("visibility.showAll", "KeyH", { alt: true }),
  binding("visibility.hideOthers", "KeyH", { shift: true }),
  binding("visibility.isolate", "NumpadDivide"),
  binding("view.front", "Numpad1"),
  binding("view.back", "Numpad1", { primary: true }),
  binding("view.right", "Numpad3"),
  binding("view.left", "Numpad3", { primary: true }),
  binding("view.top", "Numpad7"),
  binding("view.bottom", "Numpad7", { primary: true }),
  binding("view.projection", "Numpad5"),
  binding("view.fitSelection", "NumpadDecimal"),
  binding("view.fitAll", "Home"),
  binding("view.toolbar", "KeyT"),
  binding("view.sidebar", "KeyN"),
  binding("view.shadingPie", "KeyZ"),
  binding("view.xray", "KeyZ", { alt: true }),
  binding("snap.pie", "KeyS", { shift: true }),
  binding("transform.confirm", "Enter", {}, "modal:transform"),
  binding("transform.cancel", "Escape", {}, "modal:transform"),
  ...["X", "Y", "Z"].flatMap((axis) => [
    binding(`transform.axis${axis}`, `Key${axis}`, {}, "modal:transform"),
    binding(
      `transform.plane${axis}`,
      `Key${axis}`,
      { shift: true },
      "modal:transform",
    ),
  ]),
];

export const navigationPresets = {
  forma: { orbit: "left", pan: "right", shiftMiddlePan: false },
  blender: { orbit: "middle", pan: "shiftMiddle", shiftMiddlePan: true },
} as const;

export function chordKey(chord: KeyChord): string {
  return `${chord.primary ? "P+" : ""}${chord.shift ? "S+" : ""}${chord.alt ? "A+" : ""}${chord.code}`;
}
export function bindingConflicts(bindings: readonly KeyBinding[]): string[] {
  const seen = new Set<string>(),
    conflicts = new Set<string>();
  for (const item of bindings) {
    const key = `${item.context}:${chordKey(item.chord)}`;
    if (seen.has(key)) conflicts.add(key);
    seen.add(key);
  }
  return [...conflicts];
}

/** Entire replacement is validated before applying; malformed profiles never steal input. */
export function readKeymap(value: unknown): KeyBinding[] | null {
  if (!Array.isArray(value) || value.length > 128) return null;
  const known = new Set(
    defaultBindings.map((item) => `${item.context}:${item.commandId}`),
  );
  const bindings: KeyBinding[] = [];
  for (const item of value) {
    if (
      !item ||
      typeof item !== "object" ||
      !known.has(`${item.context}:${item.commandId}`)
    )
      return null;
    const chord = item.chord;
    if (
      !chord ||
      typeof chord.code !== "string" ||
      !/^(Key[A-Z]|Digit[0-9]|Numpad(?:[0-9]|Divide|Decimal)|F(?:[1-9]|1[0-2])|Home|End|Space|Tab|Enter|Escape|Delete|Backspace)$/.test(
        chord.code,
      )
    )
      return null;
    if (
      [chord.primary, chord.shift, chord.alt].some(
        (flag) => flag !== undefined && typeof flag !== "boolean",
      )
    )
      return null;
    bindings.push({
      commandId: item.commandId,
      context: item.context,
      chord: {
        code: chord.code,
        primary: !!chord.primary,
        shift: !!chord.shift,
        alt: !!chord.alt,
      },
    });
  }
  return bindingConflicts(bindings).length ? null : bindings;
}
