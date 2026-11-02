import { useEffect, useRef, useState } from "react";
import { t } from "../../i18n";
import "./mainMenu.css";
export interface MenuAction {
  id: string;
  label: string;
  run: () => void;
  shortcut?: string;
  disabled?: boolean;
}
export interface MenuGroup {
  id: string;
  label: string;
  actions: MenuAction[];
}
export function MainMenu({ groups }: { groups: MenuGroup[] }) {
  const [open, setOpen] = useState<string | null>(null),
    root = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(null);
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, []);
  return (
    <div
      className="main-menu"
      role="menubar"
      aria-label={t("Main menu")}
      ref={root}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          setOpen(null);
          root.current
            ?.querySelector<HTMLButtonElement>(`[data-menu-trigger="${open}"]`)
            ?.focus();
        }
        if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
          const items = Array.from(
            root.current?.querySelectorAll<HTMLButtonElement>(
              '[role="menu"] [role="menuitem"]:not(:disabled)',
            ) ?? [],
          );
          if (!items.length) return;
          event.preventDefault();
          const index = items.indexOf(
            document.activeElement as HTMLButtonElement,
          );
          items[
            event.key === "Home"
              ? 0
              : event.key === "End"
                ? items.length - 1
                : (index + (event.key === "ArrowUp" ? -1 : 1) + items.length) %
                  items.length
          ]?.focus();
        }
        if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
          event.preventDefault();
          const triggers = Array.from(
            root.current?.querySelectorAll<HTMLButtonElement>(
              "[data-menu-trigger]",
            ) ?? [],
          );
          const current = triggers.findIndex(
            (button) =>
              button.dataset.menuTrigger === open ||
              button === document.activeElement,
          );
          const next =
            triggers[
              (current +
                (event.key === "ArrowRight" ? 1 : -1) +
                triggers.length) %
                triggers.length
            ];
          if (next) {
            next.focus();
            if (open) setOpen(next.dataset.menuTrigger!);
          }
        }
      }}
    >
      {groups.map((group) => (
        <div className="main-menu-group" key={group.id}>
          <button
            role="menuitem"
            data-menu-trigger={group.id}
            aria-haspopup="menu"
            aria-expanded={open === group.id}
            onClick={() => setOpen(open === group.id ? null : group.id)}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setOpen(group.id);
                requestAnimationFrame(() =>
                  root.current
                    ?.querySelector<HTMLButtonElement>(
                      '[role="menu"] button:not(:disabled)',
                    )
                    ?.focus(),
                );
              }
            }}
          >
            {t(group.label)}
          </button>
          {open === group.id && (
            <div role="menu" aria-label={t(group.label)}>
              {group.actions.map((action) => (
                <button
                  role="menuitem"
                  disabled={action.disabled}
                  key={action.id}
                  onClick={() => {
                    setOpen(null);
                    action.run();
                  }}
                >
                  {action.label}
                  {action.shortcut && <kbd>{action.shortcut}</kbd>}
                </button>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
