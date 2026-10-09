import { expect, it } from "vitest";
import { hasImportedSource, importStepHistory } from "./importStepHistory";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
const file = { name: "original.STP", sha256: "a".repeat(64), size: 100, kind: "model" as const };
it("enables old imported STEP by its exact asset rather than fabricating an editable profile", () => {
  const source = importStepHistory(file, "rev")!; const document = readTypedCadDocument(source)!;
  expect(document.features).toHaveLength(1); expect(document.features[0].operation).toEqual({ type: "importStep", assetId: `step_${file.sha256}`, sha256: file.sha256 });
  expect(hasImportedSource(document, file)).toBe(true);
  expect(hasImportedSource(document, { ...file, sha256: "b".repeat(64) })).toBe(false);
});
it("does not invent a CAD source for mesh or unverified input", () => {
  expect(importStepHistory({ ...file, name: "mesh.stl" }, "rev")).toBeNull();
  expect(importStepHistory({ ...file, sha256: undefined }, "rev")).toBeNull();
  expect(importStepHistory({ ...file, sha256: "../../invalid" }, "rev")).toBeNull();
});
