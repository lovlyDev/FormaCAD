import { describe, expect, it } from "vitest";
import type { Project } from "../../types";
import { PartialCatalogError, requireReadyProjects, type CatalogReport } from "./contract";
import { errorText, setLocale } from "../../i18n";

describe("strict catalog compatibility", () => {
  it.each(["ru", "en"] as const)("localizes native catalog errors in %s", locale => {
    setLocale(locale);
    try {
      for (const code of ["PROJECT_CATALOG_LIMIT", "PROJECT_CATALOG_UNAVAILABLE"]) {
        expect(errorText(code)).not.toBe(code);
      }
    } finally { setLocale("ru"); }
  });
  it("retains exact ready Project objects", () => {
    const project = { id: "fixture" } as Project;
    expect(requireReadyProjects({ entries: [{ kind: "ready", rowKey: "managed-index:9007199254740993", project }], serializedProjectBytes: 0 })[0]).toBe(project);
  });
  it("retains unavailable rows in the error instead of dropping or fabricating them", () => {
    const report: CatalogReport = { entries: [{ kind: "unavailable", rowKey: "managed-index:9007199254740993", index: { projectId: null, idHint: "broken", name: null, updatedAt: null }, issue: { status: "recovery_required", code: "PROJECT_SNAPSHOT_RECOVERY_REQUIRED", details: "technical details" } }], serializedProjectBytes: 0 };
    try { requireReadyProjects(report); throw new Error("expected rejection"); } catch (error) {
      expect(error).toBeInstanceOf(PartialCatalogError);
      expect((error as PartialCatalogError).report).toBe(report);
      expect((error as Error).message).not.toContain("technical details");
    }
  });
});
