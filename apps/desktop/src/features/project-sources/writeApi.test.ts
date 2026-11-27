import { afterEach, expect, it, vi } from "vitest";
import { executeFolderWrite } from "./writeApi";

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
const approvalId = "0fdca3bb-60ac-4014-99e9-28b8306687c7";
const envelope = { projectId, sourceId, baseCommitId };
const project = {
  schemaVersion: 1, id: projectId, name: "Original", currentRevision: null,
  units: "mm", agent: "codex", pinned: false,
  createdAt: "2026-11-20T00:00:00Z", updatedAt: "2026-11-20T00:00:00Z",
  revisions: [], messages: [], files: [], exports: [],
};
const ticket = () => ({
  id: approvalId,
  requiresConfirmation: true,
  projectId,
  source: { sourceId, commitId: baseCommitId, taskScopeId: sourceId },
});

it.each([
  ["program", "apply_program"],
  ["commands", "apply_ir_commands"],
  ["undo", "undo_model"],
  ["redo", "redo_model"],
] as const)("executes %s with the retained approval and no new payload", async (command, ipc) => {
  mocks.invoke.mockResolvedValue({ ...envelope, status: "committed", commitId, project });
  expect(await executeFolderWrite(ticket(), command)).toEqual({
    ...envelope, status: "committed", commitId, project,
  });
  expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith(ipc, {
    projectId, folderWrite: { sourceId, expectedCommitId: baseCommitId, approvalId },
  });
});

it.each([
  { ...envelope, status: "noOp", project },
  { ...envelope, status: "rejectedBefore", error: "PROJECT_SNAPSHOT_CHANGED" },
  { ...envelope, status: "indeterminate", commitId, error: "PROJECT_SNAPSHOT_CHANGED" },
])("preserves the host's $status result without retrying", async (response) => {
  mocks.invoke.mockResolvedValue(response);
  expect(await executeFolderWrite(ticket(), "program")).toEqual(response);
  expect(mocks.invoke).toHaveBeenCalledTimes(1);
});

it.each([
  project,
  { ...envelope, sourceId: commitId, status: "noOp", project },
  { ...envelope, baseCommitId: commitId, status: "noOp", project },
  { ...envelope, status: "committed", commitId: baseCommitId, project },
  { ...envelope, status: "committed", commitId, project: { ...project, id: commitId } },
  { ...envelope, status: "indeterminate", error: "uncertain" },
])("rejects a malformed or foreign reply", async (response) => {
  mocks.invoke.mockResolvedValue(response);
  await expect(executeFolderWrite(ticket(), "program")).rejects.toThrow("PROJECT_SNAPSHOT_CHANGED");
  expect(mocks.invoke).toHaveBeenCalledTimes(1);
});

it("matches the result against the original ticket across the IPC await", async () => {
  let finish!: (value: unknown) => void;
  mocks.invoke.mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
  const selected = ticket();
  const pending = executeFolderWrite(selected, "program");
  selected.source.commitId = commitId;
  finish({ ...envelope, status: "noOp", project });
  expect(await pending).toMatchObject({ baseCommitId, status: "noOp" });
});

it("rejects unsupported clients and malformed tickets before invoking the host", async () => {
  mocks.isTauri.mockReturnValueOnce(false);
  await expect(executeFolderWrite(ticket(), "undo")).rejects.toThrow("PROJECT_FOLDER_UNSUPPORTED");
  await expect(executeFolderWrite({ ...ticket(), id: "invalid" }, "undo")).rejects.toThrow("PROJECT_SNAPSHOT_CHANGED");
  expect(mocks.invoke).not.toHaveBeenCalled();
});
