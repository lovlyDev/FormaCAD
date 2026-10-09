import { afterEach, expect, it } from "vitest";
import { errorText, rawError, setLocale, t } from "./index";
afterEach(() => setLocale("ru"));
it.each(["ru", "en"] as const)("localizes kernel reference failures in %s and retains source diagnostics separately", locale => {
  setLocale(locale);
  const diagnostic = JSON.stringify({ code: "TOPOLOGY_REFERENCE_UNRESOLVED", featureId: "rounded_corner", message: "Edge owner or occurrence is absent", detail: { ownerFeatureId: "pad", path: ["rotation"] } });
  expect(errorText(new Error(diagnostic))).toBe(`rounded_corner · TOPOLOGY_REFERENCE_UNRESOLVED: ${t("TOPOLOGY_REFERENCE_UNRESOLVED")}`);
  expect(rawError(new Error(diagnostic))).toBe(diagnostic);
});
it("preserves unknown diagnostic messages and user file names", () => {
  const message = "source Part-A.step cannot be read";
  expect(errorText(JSON.stringify({ code: "CUSTOM_EXTERNAL_FAILURE", message }))).toBe(`CUSTOM_EXTERNAL_FAILURE: ${message}`);
  expect(errorText(new Error("Part-A.step"))).toBe("Part-A.step");
});
it.each(["ru", "en"] as const)("localizes pure project snapshot failures in %s without exposing storage diagnostics", locale => {
  setLocale(locale);
  for (const code of ["PROJECT_SNAPSHOT_MISSING", "PROJECT_SNAPSHOT_CHANGED", "PROJECT_SNAPSHOT_RECOVERY_REQUIRED"]) {
    const diagnostic = JSON.stringify({ code, message: "Native storage diagnostic: generation is absent", detail: { path: "private-project-path", expectedGeneration: 7 } });
    const displayed = errorText(new Error(diagnostic));
    expect(t(code)).not.toBe(code);
    // AppError::Invalid is serialized as a plain string by the native host.
    expect(errorText(code)).toBe(t(code));
    expect(errorText(new Error(code))).toBe(t(code));
    expect(displayed).toBe(`${code}: ${t(code)}`);
    expect(displayed).not.toContain("private-project-path");
    expect(displayed).not.toContain("Native storage diagnostic");
    expect(rawError(new Error(diagnostic))).toBe(diagnostic);
  }
});
