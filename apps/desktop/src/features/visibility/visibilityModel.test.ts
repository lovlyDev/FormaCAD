import { describe, expect, it } from "vitest";
import {
  bodyVisible,
  changeVisibility,
  readVisibility,
} from "./visibilityModel";
describe("non-destructive visibility", () => {
  it("restores prior hidden bodies after isolation and preserves explicit changes during isolation", () => {
    const ids = ["a", "b", "c"];
    let state = changeVisibility(
      { hidden: ["c"], isolated: null },
      ids,
      ["a"],
      "isolate",
    );
    expect(ids.filter((id) => bodyVisible(state, id))).toEqual(["a"]);
    state = changeVisibility(state, ids, ["a"], "hide");
    state = changeVisibility(state, ids, [], "isolate");
    expect(ids.filter((id) => bodyVisible(state, id))).toEqual(["b"]);
    expect(changeVisibility(state, ids, [], "showAll")).toEqual({
      hidden: [],
      isolated: null,
    });
  });
  it("never hides everything because selection is empty, and prunes stale body IDs", () => {
    expect(readVisibility({ isolated: false }, ["a"])).toEqual({
      hidden: [],
      isolated: null,
    });
    expect(readVisibility({ isolated: ["old"] }, ["a"]).isolated).toBeNull();
    expect(
      changeVisibility(
        { hidden: [], isolated: null },
        ["a", "b"],
        [],
        "hideOthers",
      ).hidden,
    ).toEqual([]);
    expect(
      readVisibility({ hidden: ["a", "a", "old", 7], isolated: ["b", "old"] }, [
        "a",
        "b",
      ]),
    ).toEqual({ hidden: ["a"], isolated: ["b"] });
  });
});
