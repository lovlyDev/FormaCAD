import { t } from "../../i18n";
import type { Project } from "../../types";

export type CatalogEntry =
  | { kind: "ready"; rowKey: string; project: Project }
  | { kind: "unavailable"; rowKey: string; index: {
      projectId: string | null; idHint: string | null; name: string | null; updatedAt: string | null;
    }; issue: { status: "recovery_required" | "changed" | "unavailable" | "limit"; code: string; details: string } };
export interface CatalogReport { entries: CatalogEntry[]; serializedProjectBytes: number }

// Transitional callers require complete authoritative Projects. Never silently
// hide an unavailable row or manufacture a Project from display-only index hints.
export class PartialCatalogError extends Error {
  constructor(readonly report: CatalogReport) {
    super(t("Some projects are unavailable. Refresh the catalog or explicitly reopen projects that require recovery."));
    this.name = "PartialCatalogError";
  }
}
export function requireReadyProjects(report: CatalogReport): Project[] {
  if (report.entries.some(entry => entry.kind === "unavailable")) throw new PartialCatalogError(report);
  return report.entries.flatMap(entry => entry.kind === "ready" ? [entry.project] : []);
}
