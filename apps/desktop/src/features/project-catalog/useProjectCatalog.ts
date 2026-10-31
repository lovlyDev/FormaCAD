import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { listProjectCatalog } from "./api";
import type { CatalogEntry } from "./contract";

export type UnavailableEntry = Extract<CatalogEntry, { kind: "unavailable" }>;

export function useProjectCatalog() {
  const catalog = useQuery({ queryKey: ["project-catalog"], queryFn: listProjectCatalog });
  const data = useMemo(() => catalog.data?.entries.flatMap(entry => entry.kind === "ready" ? [entry.project] : []), [catalog.data]);
  const unavailable = useMemo(() => catalog.data?.entries.filter((entry): entry is UnavailableEntry => entry.kind === "unavailable") ?? [], [catalog.data]);
  // Ready snapshots provide display metadata. Navigation always reads its target.
  return { catalog, projects: { data }, unavailable };
}
