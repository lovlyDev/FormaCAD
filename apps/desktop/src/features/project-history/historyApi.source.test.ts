import { afterEach, expect, it, vi } from "vitest";
import { modelHistoryStatus } from "./historyApi";

const mock = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => mock);
afterEach(() => mock.invoke.mockReset());

const source = { sourceId: "scope:1:nonce", commitId: "stored", taskScopeId: "tasks" };
const status = { canUndo: true, canRedo: false, expectedRevision: "revision" };

it("keeps the managed request shape and reads folder history at an exact commit", async () => {
  mock.invoke.mockResolvedValue(status);
  expect(await modelHistoryStatus("project")).toEqual(status);
  expect(await modelHistoryStatus("project", source)).toEqual(status);
  expect(mock.invoke.mock.calls).toEqual([
    ["history_status", { projectId: "project" }],
    ["history_status", { projectId: "project", sourceId: source.sourceId, expectedCommitId: source.commitId }],
  ]);
});

it("does not fall back to managed history for an incomplete folder selector", async () => {
  await expect(modelHistoryStatus("project", { ...source, commitId: "" }))
    .rejects.toThrow("PROJECT_SNAPSHOT_CHANGED");
  expect(mock.invoke).not.toHaveBeenCalled();
});

it("does not retry against managed history if the folder snapshot is stale", async () => {
  mock.invoke.mockRejectedValue(new Error("PROJECT_SNAPSHOT_CHANGED"));
  await expect(modelHistoryStatus("project", source)).rejects.toThrow("PROJECT_SNAPSHOT_CHANGED");
  expect(mock.invoke).toHaveBeenCalledTimes(1);
});
