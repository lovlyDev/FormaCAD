import { beforeEach, expect, it, vi } from "vitest";
import { t } from "../../i18n";
import { newProject, useWorkspace } from "../../stores/workspace";
import { saveFolderCopy } from "./folderCopy";

const mock = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", async original => ({
  ...await original<typeof import("@tauri-apps/api/core")>(),
  invoke: mock.invoke,
  isTauri: () => true,
}));

const source = { sourceId: "window:1:nonce", commitId: "source-commit", taskScopeId: "task" };

beforeEach(() => {
  useWorkspace.getState().setProject(null);
  mock.invoke.mockReset();
});

function setup() {
  const project = newProject("Folder", "blank", "mm", "codex");
  useWorkspace.getState().openFolderProject(project, source);
  const opening = useWorkspace.getState().sourceBinding;
  if (opening?.kind !== "folder") throw new Error("Expected folder");
  const saved = vi.fn(), error = vi.fn();
  return { project, opening, saved, error, copy: () => saveFolderCopy(opening, saved, error) };
}

it("sends the captured source, keeps it open and reports the independent copy", async () => {
  const f = setup();
  mock.invoke.mockResolvedValueOnce({ status: "saved", commitId: "copy-commit" });
  await f.copy();
  expect(mock.invoke.mock.calls).toEqual([["save_project_folder_copy", {
    projectId: f.project.id, sourceId: source.sourceId, expectedCommitId: source.commitId,
  }]]);
  expect(useWorkspace.getState().sourceBinding).toBe(f.opening);
  expect(f.saved).toHaveBeenCalledExactlyOnceWith(t("Project copy saved"));
  expect(f.error).not.toHaveBeenCalled();
});

it("leaves the current source alone on picker cancel", async () => {
  const f = setup();
  mock.invoke.mockResolvedValueOnce(null);
  await f.copy();
  expect(f.saved).not.toHaveBeenCalled();
  expect(f.error).not.toHaveBeenCalled();
  expect(useWorkspace.getState().sourceBinding).toBe(f.opening);
});

it("reports an uncertain destination commit without retrying", async () => {
  const f = setup();
  mock.invoke.mockResolvedValueOnce({ status: "indeterminate", commitId: "attempt", error: "Lost response" });
  await f.copy();
  expect(f.saved).not.toHaveBeenCalled();
  expect(f.error).toHaveBeenCalledWith(expect.stringContaining("attempt\nLost response"));
  expect(mock.invoke).toHaveBeenCalledOnce();
});

it("drops a late response after switching to another opening of the same project", async () => {
  const f = setup();
  mock.invoke.mockResolvedValueOnce({ status: "saved", commitId: "copy-commit" });
  const pending = f.copy();
  useWorkspace.getState().openFolderProject(f.project, { ...source, sourceId: "new-window:1:nonce" });
  await pending;
  expect(f.saved).not.toHaveBeenCalled();
  expect(f.error).not.toHaveBeenCalled();
});
