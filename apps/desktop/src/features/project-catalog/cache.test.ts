import { QueryClient } from "@tanstack/react-query";
import { expect, it } from "vitest";
import { invalidateProjectCatalog } from "./cache";
it("invalidates both read views without clearing the authoritative Project cache", () => {
  const query = new QueryClient(), snapshot = [{ id: "active" }];
  query.setQueryData(["projects"], snapshot);
  query.setQueryData(["project-catalog"], { entries: [] });
  invalidateProjectCatalog(query);
  expect(query.getQueryData(["projects"])).toBe(snapshot);
  expect(query.getQueryState(["projects"])?.isInvalidated).toBe(true);
  expect(query.getQueryState(["project-catalog"])?.isInvalidated).toBe(true);
  query.clear();
});
