import { useEffect, useRef } from "react";
import { defaultBindings } from "./registry";
import { ownsTextInput, resolveKeymap } from "./resolveKeymap";

export type ViewportCommands = Partial<Record<string, () => void>>;

/**
 * Single keyboard route for one 3D viewport. Only commands with handlers are
 * available, the viewport area must own focus, and text fields keep their keys.
 * The modal transform installs its own capture listener and wins while active.
 */
export function useViewportKeymap(
  commands: ViewportCommands,
  options: { enabled: boolean; emulateNumpad: boolean },
) {
  const latest = useRef(commands);
  useEffect(() => {
    latest.current = commands;
  }, [commands]);
  useEffect(() => {
    if (!options.enabled) return;
    const keydown = (event: KeyboardEvent) => {
      const target = event.target;
      if (
        !(target instanceof Element) ||
        !target.closest('[data-editor="viewport3d"]')
      )
        return;
      const handlers = latest.current;
      const available = new Set(
        Object.keys(handlers).filter((id) => handlers[id] !== undefined),
      );
      const command = resolveKeymap(event, defaultBindings, {
        context: "viewport",
        textInput: ownsTextInput(target),
        emulateNumpad: options.emulateNumpad,
        available,
      });
      if (!command) return;
      event.preventDefault();
      handlers[command]?.();
    };
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  }, [options.enabled, options.emulateNumpad]);
}
