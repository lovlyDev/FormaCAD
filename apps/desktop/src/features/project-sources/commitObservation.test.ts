import { afterEach, expect, it, vi } from "vitest";
import { observeFolderCommit } from "./commitObservation";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock("@tauri-apps/api/core", () => mocks);
afterEach(() => {
  mocks.invoke.mockReset();
  mocks.isTauri.mockReset().mockReturnValue(true);
});

const projectId = "9ef4f985-695a-4a73-b8de-b10468bfc450";
const sourceId = "340f0083-c50f-4e15-b633-5cc4753ffb5c:1:8c45ccab-969f-4b94-a565-73db75eae0ef";
const baseCommitId = "78b94346-f162-44e8-85c6-9cb64b8be739";
const commitId = "7c6c3844-bdbe-4132-91ff-8e52a46a97ec";
const source = () => ({ sourceId, commitId: baseCommitId, taskScopeId: projectId });
const query = { projectId, sourceId, commitId };

it.each(["current", "historical", "absent"])("returns %s from one read", async (status) => {
  const reply = { ...query, status };
  mocks.invoke.mockResolvedValue(reply);
  expect(await observeFolderCommit(projectId, source(), commitId)).toEqual(reply);
  expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith("observe_folder_commit", query);
});

it.each([
  { ...query, sourceId: "another source", status: "current" },
  { ...query, projectId: baseCommitId, status: "current" },
  { ...query, commitId: baseCommitId, status: "current" },
  { ...query, status: "replay" },
])("rejects an unrelated or malformed reply", async (reply) => {
  mocks.invoke.mockResolvedValue(reply);
  await expect(observeFolderCommit(projectId, source(), commitId))
    .rejects.toThrow("PROJECT_SNAPSHOT_CHANGED");
  expect(mocks.invoke).toHaveBeenCalledTimes(1);
});

it("tracks the source at request time while the lookup is in flight", async () => {
  let finish!: (value: unknown) => void;
  mocks.invoke.mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
  const opened = source();
  const answer = observeFolderCommit(projectId, opened, commitId);
  opened.sourceId = "new opening";
  finish({ ...query, status: "historical" });
  expect(await answer).toMatchObject({ sourceId, status: "historical" });
});

it("does not query for unsupported hosts or malformed commit IDs", async () => {
  mocks.isTauri.mockReturnValueOnce(false);
  await expect(observeFolderCommit(projectId, source(), commitId))
    .rejects.toThrow("PROJECT_FOLDER_UNSUPPORTED");
  await expect(observeFolderCommit(projectId, source(), "unknown"))
    .rejects.toThrow("PROJECT_SNAPSHOT_CHANGED");
  expect(mocks.invoke).not.toHaveBeenCalled();
});
