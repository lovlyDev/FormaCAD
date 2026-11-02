import type { Area, LayoutNode, Split } from "./layoutModel";
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface PlacedArea extends Rect {
  area: Area;
}
export interface PlacedSplit extends Rect {
  node: Split;
  parent: Rect;
  minRatio: number;
  maxRatio: number;
}
export const separatorSize = 6;
const minimum = { width: 180, height: 120 };
export function minimumSize(node: LayoutNode): {
  width: number;
  height: number;
} {
  if (node.kind === "area") return minimum;
  const a = minimumSize(node.first),
    b = minimumSize(node.second);
  return node.axis === "x"
    ? {
        width: a.width + b.width + separatorSize,
        height: Math.max(a.height, b.height),
      }
    : {
        width: Math.max(a.width, b.width),
        height: a.height + b.height + separatorSize,
      };
}
export function placeLayout(root: LayoutNode, width: number, height: number) {
  const placed: PlacedArea[] = [],
    separators: PlacedSplit[] = [];
  function visit(node: LayoutNode, rect: Rect) {
    if (node.kind === "area") {
      placed.push({ ...rect, area: node });
      return;
    }
    const isX = node.axis === "x",
      available = Math.max(0, (isX ? rect.width : rect.height) - separatorSize);
    const firstMin = minimumSize(node.first),
      secondMin = minimumSize(node.second);
    const lo = Math.min(
      0.9,
      (isX ? firstMin.width : firstMin.height) / Math.max(1, available),
    );
    const hi = Math.max(
      0.1,
      1 - (isX ? secondMin.width : secondMin.height) / Math.max(1, available),
    );
    const ratio = lo <= hi ? Math.max(lo, Math.min(hi, node.ratio)) : 0.5;
    const first = available * ratio,
      second = available - first;
    separators.push({
      node: { ...node, ratio },
      parent: rect,
      minRatio: Math.min(lo, hi),
      maxRatio: Math.max(lo, hi),
      x: rect.x + (isX ? first : 0),
      y: rect.y + (isX ? 0 : first),
      width: isX ? separatorSize : rect.width,
      height: isX ? rect.height : separatorSize,
    });
    visit(node.first, {
      ...rect,
      width: isX ? first : rect.width,
      height: isX ? rect.height : first,
    });
    visit(node.second, {
      x: rect.x + (isX ? first + separatorSize : 0),
      y: rect.y + (isX ? 0 : first + separatorSize),
      width: isX ? second : rect.width,
      height: isX ? rect.height : second,
    });
  }
  visit(root, { x: 0, y: 0, width, height });
  return { areas: placed, separators };
}
