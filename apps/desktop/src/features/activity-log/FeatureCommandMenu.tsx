import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { t } from "../../i18n";
import type { TypedCadFeature } from "../../lib/typedCadDocument";
import { copyFeatureCommand } from "./copyFeatureCommand";
import "./featureCommandMenu.css";

export type FeatureMenuTarget = {
  feature: TypedCadFeature;
  anchor: HTMLButtonElement;
  x: number;
  y: number;
};

export function FeatureCommandMenu({ target, close }: { target: FeatureMenuTarget; close: () => void }) {
  const menu = useRef<HTMLDivElement>(null);
  const action = useRef<HTMLButtonElement>(null);
  const [failed, setFailed] = useState(false);
  const { anchor, feature, x, y } = target;
  useEffect(() => {
    const element = menu.current;
    if (!element) return;
    element.style.left = `${Math.max(0, Math.min(x, window.innerWidth - element.offsetWidth))}px`;
    element.style.top = `${Math.max(0, Math.min(y, window.innerHeight - element.offsetHeight))}px`;
    action.current?.focus();
    const outside = (event: PointerEvent) => {
      if (event.target instanceof Node && !element.contains(event.target)) close();
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.stopPropagation();
      anchor.focus();
      close();
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape, true);
    window.addEventListener("resize", close);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", escape, true);
      window.removeEventListener("resize", close);
    };
  }, [anchor, close, x, y]);
  return createPortal(
    <div className="feature-command-menu" ref={menu}>
      <button
        type="button"
        ref={action}
        onClick={async () => {
          try {
            await navigator.clipboard.writeText(copyFeatureCommand(feature));
            if (!menu.current) return;
            anchor.focus();
            close();
          } catch {
            setFailed(true);
          }
        }}
      >
        {t("Copy as command")}
      </button>
      {failed && <p role="status">{t("Could not copy the command.")}</p>}
    </div>,
    document.body,
  );
}
