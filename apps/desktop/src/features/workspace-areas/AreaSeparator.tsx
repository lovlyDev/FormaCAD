import type { PointerEvent } from "react";
import { t, number } from "../../i18n";
import { separatorSize, type PlacedSplit } from "./layoutGeometry";
export function AreaSeparator({
  split,
  onResize,
}: {
  split: PlacedSplit;
  onResize: (id: string, ratio: number) => void;
}) {
  const { node, parent } = split,
    isX = node.axis === "x";
  const clamp = (value: number) =>
    Math.max(split.minRatio, Math.min(split.maxRatio, value));
  const drag = (event: PointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
  };
  return (
    <div
      role="separator"
      tabIndex={0}
      className={`workspace-area-separator axis-${node.axis}`}
      aria-label={
        isX ? t("Resize areas horizontally") : t("Resize areas vertically")
      }
      aria-orientation={isX ? "vertical" : "horizontal"}
      aria-valuemin={Math.round(split.minRatio * 100)}
      aria-valuemax={Math.round(split.maxRatio * 100)}
      aria-valuenow={Math.round(node.ratio * 100)}
      aria-valuetext={t("Area size {{value0}} percent", {
        value0: number(node.ratio * 100, 0),
      })}
      style={{
        left: split.x,
        top: split.y,
        width: split.width,
        height: split.height,
      }}
      onPointerDown={drag}
      onPointerMove={(event) => {
        if (!event.currentTarget.hasPointerCapture(event.pointerId)) return;
        const bounds =
          event.currentTarget.parentElement!.getBoundingClientRect();
        const offset = isX
          ? event.clientX - bounds.left - parent.x
          : event.clientY - bounds.top - parent.y;
        onResize(
          node.id,
          clamp(
            offset /
              Math.max(1, (isX ? parent.width : parent.height) - separatorSize),
          ),
        );
      }}
      onPointerUp={(event) => {
        if (event.currentTarget.hasPointerCapture(event.pointerId))
          event.currentTarget.releasePointerCapture(event.pointerId);
      }}
      onKeyDown={(event) => {
        const direction = isX
          ? event.key === "ArrowRight"
            ? 1
            : event.key === "ArrowLeft"
              ? -1
              : 0
          : event.key === "ArrowDown"
            ? 1
            : event.key === "ArrowUp"
              ? -1
              : 0;
        if (!direction && !["Home", "End"].includes(event.key)) return;
        event.preventDefault();
        onResize(
          node.id,
          clamp(
            event.key === "Home"
              ? split.minRatio
              : event.key === "End"
                ? split.maxRatio
                : node.ratio + direction * (event.shiftKey ? 0.1 : 0.02),
          ),
        );
      }}
    />
  );
}
