import { describe, expect, it } from "vitest";
import {
  emptySelection,
  updateSelection,
  type SelectionTarget,
} from "./selectionModel";
import { boxRegion, regionIntersects } from "./region-select/regionGeometry";
import { regionSelection } from "./region-select/regionSelection";
const scope = { projectId: "p", revisionId: "r", sceneToken: "s" };
const target = (key: string, extra = {}): SelectionTarget => ({
  key,
  bodyId: key,
  kind: "body",
  selectable: true,
  visible: true,
  ...extra,
});
describe("selection membership and active item", () => {
  it("toggles independently, keeps the last active item and inverts only eligible bodies", () => {
    const universe = [
      target("a"),
      target("b"),
      target("hidden", { visible: false }),
      target("locked", { selectable: false }),
      target("edge", { kind: "edge" }),
    ];
    let state = updateSelection(
      emptySelection(scope),
      scope,
      universe,
      ["a"],
      "replace",
      "body",
    );
    state = updateSelection(
      state,
      scope,
      universe,
      ["b", "b", "locked"],
      "add",
      "body",
    );
    expect(state.keys).toEqual(["a", "b"]);
    expect(state.active).toBe("b");
    state = updateSelection(state, scope, universe, ["b"], "toggle", "body");
    expect(state.keys).toEqual(["a"]);
    expect(state.active).toBe("a");
    state = updateSelection(state, scope, universe, [], "invert", "body");
    expect(state.keys).toEqual(["b"]);
  });
  it("invalidates the whole set on revision or scene change instead of retargeting indices", () => {
    const universe = [target("a")],
      selected = updateSelection(
        emptySelection(scope),
        scope,
        universe,
        ["a"],
        "replace",
        "body",
      );
    expect(
      updateSelection(
        selected,
        { ...scope, sceneToken: "new" },
        universe,
        ["a"],
        "add",
        "body",
      ).keys,
    ).toEqual([]);
    expect(
      updateSelection(selected, scope, universe, [], "none", "body").active,
    ).toBeNull();
  });
});
describe("screen region intersection", () => {
  it("hits edges crossing a box and faces containing the entire circle/lasso", () => {
    expect(
      regionIntersects(boxRegion([0, 0], [10, 10]), [
        [-5, 5],
        [20, 5],
      ]),
    ).toBe(true);
    expect(
      regionIntersects({ kind: "circle", center: [0, 0], radius: 1 }, [
        [-20, -20],
        [20, -20],
        [0, 20],
      ]),
    ).toBe(true);
    expect(
      regionIntersects(boxRegion([-1, -1], [1, 1]), [
        [-20, -20],
        [20, -20],
        [0, 20],
      ]),
    ).toBe(true);
    expect(
      regionIntersects(boxRegion([0, 0], [10, 10]), [
        [0, 10],
        [10, 10],
      ]),
    ).toBe(true);
    expect(
      regionIntersects({ kind: "circle", center: [0, 0], radius: 1 }, [[2, 2]]),
    ).toBe(false);
    expect(
      regionIntersects(
        {
          kind: "polygon",
          points: [
            [0, 0],
            [10, 0],
          ],
        },
        [[0, 0]],
      ),
    ).toBe(false);
  });
  it("deduplicates semantic face IDs and respects depth/visibility/selection locks", () => {
    const projected = (key: string, extra = {}) => ({
      target: target(key, { kind: "face" }),
      primitives: [[[1, 1]]] as const,
      inFrontOfCamera: true,
      occluded: false,
      ...extra,
    });
    const candidates = [
      projected("face1"),
      projected("face1"),
      projected("face2", { occluded: true }),
      projected("behind", { inFrontOfCamera: false }),
      projected("locked", {
        target: target("locked", { kind: "face", selectable: false }),
      }),
    ];
    expect(
      regionSelection(boxRegion([0, 0], [2, 2]), candidates, "face", false),
    ).toEqual(["face1"]);
    expect(
      regionSelection(boxRegion([0, 0], [2, 2]), candidates, "face", true),
    ).toEqual(["face1", "face2"]);
  });
});
