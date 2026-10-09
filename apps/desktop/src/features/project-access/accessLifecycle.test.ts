import { afterEach, beforeEach, expect, it, vi } from "vitest";
const api = vi.hoisted(() => ({ acquire: vi.fn(), release: vi.fn() }));
vi.mock("./projectAccessApi", () => ({ acquireProjectAccess: api.acquire, releaseProjectAccess: api.release }));
beforeEach(() => { vi.resetModules(); vi.useFakeTimers(); api.acquire.mockReset().mockResolvedValue({ projectId: "project", mode: "write", ownerPid: 1 }); api.release.mockReset().mockResolvedValue(undefined); });
afterEach(() => vi.useRealTimers());
it("keeps a lease through StrictMode cleanup/remount and releases only the last viewer", async () => {
  const { retainAccess, acquireAccess } = await import("./accessLifecycle");
  const first = retainAccess("project", vi.fn()); await acquireAccess("project"); first();
  const second = retainAccess("project", vi.fn()); await vi.advanceTimersByTimeAsync(300);
  expect(api.release).not.toHaveBeenCalled(); second(); await vi.advanceTimersByTimeAsync(300);
  expect(api.release).toHaveBeenCalledExactlyOnceWith("project");
});
it("serializes acquiring a view after an in-flight release", async () => {
  const { retainAccess, acquireAccess } = await import("./accessLifecycle");
  let finish!: () => void; api.release.mockImplementation(() => new Promise<void>(resolve => { finish = resolve; }));
  const release = retainAccess("project", vi.fn()); await acquireAccess("project"); release(); await vi.advanceTimersByTimeAsync(300);
  const second = retainAccess("project", vi.fn()); const acquired = acquireAccess("project");
  await Promise.resolve(); expect(api.acquire).toHaveBeenCalledTimes(1);
  finish(); await acquired; expect(api.acquire).toHaveBeenCalledTimes(2);
  api.release.mockResolvedValue(undefined); second(); await vi.advanceTimersByTimeAsync(300);
});
it("reports a failed release instead of silently claiming the project was unlocked", async () => {
  const { retainAccess } = await import("./accessLifecycle"); const error = new Error("PROJECT_ACCESS_BUSY"); api.release.mockRejectedValue(error);
  const report = vi.fn(); retainAccess("project", report)(); await vi.advanceTimersByTimeAsync(300);
  expect(report).toHaveBeenCalledWith(error);
});
