import { useEffect, useRef } from "react";
import { create } from "zustand";
import { t } from "../../i18n";
import "./pieMenu.css";

export interface PieItem {
  id: string;
  label: string;
  active?: boolean;
  disabled?: boolean;
  run: () => void;
}
interface PieState {
  open: { title: string; items: PieItem[]; x: number; y: number } | null;
  show: (title: string, items: PieItem[], x: number, y: number) => void;
  close: () => void;
}
export const usePieMenu = create<PieState>((set) => ({
  open: null,
  show: (title, items, x, y) => set({ open: { title, items, x, y } }),
  close: () => set({ open: null }),
}));

const radius = 92;
/** Item positions start at the top and go clockwise, so number keys keep a stable order. */
export function piePosition(index: number, count: number) {
  const angle = -Math.PI / 2 + (index * 2 * Math.PI) / Math.max(count, 1);
  return { x: Math.cos(angle) * radius, y: Math.sin(angle) * radius };
}

/** Radial launcher: click an item or press its number; Esc or a click outside closes. */
export function PieMenu() {
  const open = usePieMenu((state) => state.open);
  const close = usePieMenu((state) => state.close);
  const first = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    if (!open) return;
    first.current?.focus();
    const key = (event: KeyboardEvent) => {
      const number = /^(?:Digit|Numpad)([1-9])$/.exec(event.code);
      const item = number ? open.items[Number(number[1]) - 1] : undefined;
      if (event.code === "Escape" || item) {
        event.preventDefault();
        event.stopImmediatePropagation();
        close();
        if (item && !item.disabled) item.run();
      }
    };
    const down = (event: PointerEvent) => {
      if (
        !(event.target instanceof Element) ||
        !event.target.closest(".pie-menu")
      )
        close();
    };
    window.addEventListener("keydown", key, true);
    window.addEventListener("pointerdown", down, true);
    return () => {
      window.removeEventListener("keydown", key, true);
      window.removeEventListener("pointerdown", down, true);
    };
  }, [open, close]);
  if (!open) return null;
  const x = Math.min(
    Math.max(open.x, radius + 80),
    window.innerWidth - radius - 80,
  );
  const y = Math.min(
    Math.max(open.y, radius + 30),
    window.innerHeight - radius - 30,
  );
  return (
    <div
      className="pie-menu"
      role="menu"
      aria-label={open.title}
      style={{ left: x, top: y }}
    >
      <div className="pie-menu-title">{open.title}</div>
      {open.items.map((item, index) => {
        const position = piePosition(index, open.items.length);
        return (
          <button
            key={item.id}
            ref={index === 0 ? first : undefined}
            role="menuitemradio"
            aria-checked={!!item.active}
            disabled={item.disabled}
            title={t("Press {{value0}}", { value0: index + 1 })}
            style={{
              transform: `translate(calc(${position.x}px - 50%), calc(${position.y}px - 50%))`,
            }}
            onClick={() => {
              close();
              item.run();
            }}
          >
            <kbd>{index + 1}</kbd> {item.label}
          </button>
        );
      })}
    </div>
  );
}
