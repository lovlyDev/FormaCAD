import { describe, it, expect } from "vitest";
import {
  areas,
  changeEditor,
  defaultLayout,
  joinArea,
  parseLayout,
  resizeSplit,
  splitArea,
  type LayoutNode,
} from "./layoutModel";
import { minimumSize, placeLayout } from "./layoutGeometry";
describe("workspace area persistence and identity", () => {
  it("keeps viewport area identity through splitting, editor swaps and joining", () => {
    const original = defaultLayout().root;
    const viewport = areas(original).find(
      (area) => area.editor === "viewport3d",
    )!;
    const divided = splitArea(original, viewport.id, "y", "new-area");
    expect(
      areas(divided).find((area) => area.editor === "viewport3d")?.id,
    ).toBe(viewport.id);
    const exchanged = changeEditor(divided, "new-area", "ai");
    expect(
      areas(exchanged).filter((area) => area.editor === "ai"),
    ).toHaveLength(1);
    const joined = joinArea(exchanged, "new-area");
    expect(areas(joined).find((area) => area.editor === "viewport3d")?.id).toBe(
      viewport.id,
    );
    expect(
      parseLayout(
        JSON.parse(JSON.stringify({ layoutVersion: 1, root: joined })),
      ),
    ).toEqual({ layoutVersion: 1, root: joined });
  });
  it("rejects corrupt and future layouts as a whole", () => {
    const fallback = defaultLayout("inspect");
    const a = { kind: "area", id: "repeated", editor: "viewport3d" };
    const invalid = [
      { layoutVersion: 2, root: a },
      {
        layoutVersion: 1,
        root: {
          kind: "split",
          id: "root",
          axis: "x",
          ratio: 0.5,
          first: a,
          second: a,
        },
      },
      { layoutVersion: 1, root: { ...a, editor: "unsupported" } },
      {
        layoutVersion: 1,
        root: {
          kind: "split",
          id: "root",
          axis: "x",
          ratio: NaN,
          first: a,
          second: { ...a, id: "other", editor: "files" },
        },
      },
    ];
    for (const value of invalid)
      expect(parseLayout(value, fallback)).toBe(fallback);
  });
  it("bounds growth to live editors and refuses duplicate identifiers", () => {
    let root = defaultLayout().root;
    expect(splitArea(root, areas(root)[0].id, "x", areas(root)[1].id)).toBe(
      root,
    );
    for (let index = 0; index < 40; index++)
      root = splitArea(root, areas(root)[0].id, "x", `new-${index}`);
    expect(areas(root)).toHaveLength(10);
    expect(new Set(areas(root).map((area) => area.editor)).size).toBe(10);
    expect(parseLayout({ layoutVersion: 1, root }).root).toEqual(root);
  });
  it("never removes the last area and ignores missing targets", () => {
    const root: LayoutNode = { kind: "area", id: "sole", editor: "viewport3d" };
    expect(joinArea(root, "sole")).toBe(root);
    expect(changeEditor(root, "missing", "ai")).toBe(root);
    expect(resizeSplit(root, "sole", Infinity)).toBe(root);
  });
});
describe("workspace layout geometry", () => {
  it("partitions the stage without overlap or missing surface", () => {
    const root = splitArea(
      defaultLayout().root,
      "area-viewport3d",
      "y",
      "extra",
    );
    const placed = placeLayout(root, 1440, 900);
    const occupied = [...placed.areas, ...placed.separators].reduce(
      (sum, rect) => sum + rect.width * rect.height,
      0,
    );
    expect(occupied).toBeCloseTo(1440 * 900, 5);
    for (let i = 0; i < placed.areas.length; i++)
      for (let j = i + 1; j < placed.areas.length; j++) {
        const a = placed.areas[i],
          b = placed.areas[j];
        expect(
          Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x) <=
            0.00001 ||
            Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y) <=
              0.00001,
        ).toBe(true);
      }
  });
  it("respects recursive minima even with extreme requested ratios", () => {
    const root = resizeSplit(defaultLayout().root, "main", 0.1);
    const min = minimumSize(root),
      placed = placeLayout(
        root,
        Math.max(1000, min.width),
        Math.max(700, min.height),
      );
    for (const area of placed.areas) {
      expect(area.width).toBeGreaterThanOrEqual(179.999);
      expect(area.height).toBeGreaterThanOrEqual(119.999);
    }
  });
});
