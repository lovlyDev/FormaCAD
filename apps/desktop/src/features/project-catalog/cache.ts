import type { QueryClient } from "@tanstack/react-query";

// Catalog metadata and authoritative Project snapshots use distinct caches.
// All accepted mutations/event refreshes invalidate both views in one place.
export function invalidateProjectCatalog(query: QueryClient): void {
  void query.invalidateQueries({ queryKey: ["project-catalog"] });
  void query.invalidateQueries({ queryKey: ["projects"] });
}
