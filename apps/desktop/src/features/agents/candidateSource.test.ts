import { expect, it } from "vitest";
import { candidateSourceStatus } from "./candidateSource";
const base = { id: "project_a", currentRevision: "shared_revision" };
it("accepts only the captured project and revision", () => {
  expect(candidateSourceStatus({ ...base }, base)).toBe("valid");
  expect(candidateSourceStatus({ ...base, currentRevision: "later" }, base)).toBe("revisionChanged");
});
it("rejects a copied project even when revision IDs match", () => {
  expect(candidateSourceStatus({ ...base, id: "project_b" }, base)).toBe("projectChanged");
});
it("rejects closed projects and supports the initial project revision", () => {
  expect(candidateSourceStatus(null, base)).toBe("projectChanged");
  expect(candidateSourceStatus({ id: "empty", currentRevision: null }, { id: "empty", currentRevision: null })).toBe("valid");
});
