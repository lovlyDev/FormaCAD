import { expect, it, vi } from "vitest";
import { captureApply, executeCapturedApply, applyAdoption } from "./capturedApply";
it("refuses pending approval after project or revision changes without invoking CAD", async () => {
  const request = captureApply({ id: "A", currentRevision: "base" }, "draft", "edit"), invoke = vi.fn();
  for (const current of [{ id: "B", currentRevision: "base" }, { id: "A", currentRevision: "new" }, null])
    await expect(executeCapturedApply(request, { getCurrent: () => current, invoke, adopt: vi.fn(), reconcileTarget: vi.fn(), refreshTarget: vi.fn() })).rejects.toThrow("MODEL_APPLY_BASE_CHANGED");
  expect(invoke).not.toHaveBeenCalled();
  expect(applyAdoption(request.base, request, request.base)).toBe("unchanged");
});
it("does not misreport accepted geometry when metadata save or reconciliation fails", async () => {
  let current = { id: "A", currentRevision: "base" };
  const request = captureApply(current, "draft", "edit"), refresh = vi.fn(), invoke = vi.fn(async () => {
    current = { id: "B", currentRevision: "base" }; return { id: "A", currentRevision: "accepted" };
  });
  const adopt = vi.fn();
  const result = await executeCapturedApply(request, { getCurrent: () => current, invoke, adopt,
    reconcileTarget: async () => { throw Error("fetch failed"); },
    saveCommittedMetadata: async () => { throw Error("message failed"); }, refreshTarget: refresh });
  expect(result.result.currentRevision).toBe("accepted"); expect(result.disposition).toBe("orphaned");
  expect(result.adoptionFailure).toBeInstanceOf(Error); expect(result.metadataFailure).toBeInstanceOf(Error);
  expect(refresh).toHaveBeenCalledWith("A"); expect(invoke).toHaveBeenCalledTimes(1); expect(adopt).not.toHaveBeenCalled();
});
