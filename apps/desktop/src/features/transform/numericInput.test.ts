import { describe, expect, it } from "vitest";
import { parseTransformNumber as parse } from "./numericInput";
describe("transform quantity input", () => {
  it("converts exact physical units and Russian decimal input", () => {
    expect(parse("10mm", "length")).toEqual({ status: "valid", value: 10 });
    expect(parse("1in", "length")).toEqual({ status: "valid", value: 25.4 });
    expect(parse("-1,5 см", "length")).toEqual({ status: "valid", value: -15 });
    expect(parse("180°", "angle")).toEqual({ status: "valid", value: 180 });
    expect(parse("100%", "scale")).toEqual({ status: "valid", value: 1 });
  });
  it("keeps incomplete edits separate from a confirmed zero", () => {
    expect(parse("", "length").status).toBe("empty");
    for (const text of ["-", "+", ".", "-,"])
      expect(parse(text, "length").status).toBe("incomplete");
    expect(parse("0", "length")).toEqual({ status: "valid", value: 0 });
  });
  it("rejects dimensional mixups, expressions, reflection and excessive values", () => {
    expect(parse("10mm", "angle")).toEqual({
      status: "invalid",
      reason: "unit",
    });
    expect(parse("90deg", "scale")).toEqual({
      status: "invalid",
      reason: "unit",
    });
    expect(parse("=15*2", "length")).toEqual({
      status: "invalid",
      reason: "expression",
    });
    for (const value of ["Infinity", "NaN", "1;alert(1)", "1e999", "1.2.3"])
      expect(parse(value, "length").status).toBe("invalid");
    for (const value of ["0", "-1", "1001"])
      expect(parse(value, "scale").status).toBe("invalid");
    expect(parse("11m", "length").status).toBe("invalid");
  });
});
