import { afterEach, expect, it, vi } from "vitest";
import { openFolderSelection } from "./openFolderSelection";

const native = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => native);

const project = { id: "same-project-id" };
const selected = { project, sourceId: "folder-opening", commitId: "saved", taskScopeId: "tasks" };

afterEach(() => vi.resetAllMocks());

function setup() {
  native.isTauri.mockReturnValue(true);
  return { isCurrent: vi.fn(() => true), activate: vi.fn() };
}

it("activates the selected snapshot with its window-local source and sends no path", async () => {
  const target = setup();
  native.invoke.mockResolvedValueOnce(selected);
  await openFolderSelection(target);
  expect(native.invoke.mock.calls).toEqual([["open_project_folder"]]);
  expect(target.activate).toHaveBeenCalledOnce();
  expect(target.activate).toHaveBeenCalledWith(project, {
    sourceId: "folder-opening", commitId: "saved", taskScopeId: "tasks",
  });
});

it("leaves the current opening alone when the picker is cancelled", async () => {
  const target = setup();
  native.invoke.mockResolvedValueOnce(null);
  await openFolderSelection(target);
  expect(target.activate).not.toHaveBeenCalled();
  expect(native.invoke).toHaveBeenCalledOnce();
});

it("sends a recent location only as a hint to the native picker", async () => {
  const target = setup();
  native.invoke.mockResolvedValueOnce(null);
  await openFolderSelection(target, "E:/Projects/Previous");
  expect(native.invoke).toHaveBeenCalledExactlyOnceWith("open_project_folder", {
    recentLocation: "E:/Projects/Previous",
  });
  expect(target.activate).not.toHaveBeenCalled();
});

it("closes a late opening after navigation changes even for the same project ID", async () => {
  const target = setup();
  native.invoke.mockResolvedValueOnce(selected).mockResolvedValueOnce(true);
  const opening = openFolderSelection(target);
  target.isCurrent.mockReturnValue(false);
  await opening;
  expect(target.activate).not.toHaveBeenCalled();
  expect(native.invoke).toHaveBeenLastCalledWith("close_project_source", { sourceId: "folder-opening" });
  expect(native.invoke).toHaveBeenCalledTimes(2);
});

it("ignores a late picker error after navigation changes", async () => {
  const target = setup();
  native.invoke.mockRejectedValueOnce(new Error("late error"));
  const opening = openFolderSelection(target);
  target.isCurrent.mockReturnValue(false);
  await expect(opening).resolves.toBeUndefined();
  expect(target.activate).not.toHaveBeenCalled();
});

it("surfaces an error for the current navigation", async () => {
  const target = setup();
  native.invoke.mockRejectedValueOnce(new Error("PICKER_FAILED"));
  await expect(openFolderSelection(target)).rejects.toThrow("PICKER_FAILED");
});

it("releases an opening whose reply is missing the commit", async () => {
  const target = setup();
  native.invoke.mockResolvedValueOnce({ ...selected, commitId: "" }).mockResolvedValueOnce(true);
  await expect(openFolderSelection(target)).rejects.toThrow("PROJECT_SNAPSHOT_CHANGED");
  expect(target.activate).not.toHaveBeenCalled();
  expect(native.invoke).toHaveBeenLastCalledWith("close_project_source", { sourceId: "folder-opening" });
});

it("keeps the folder picker unavailable outside the desktop host", async () => {
  const target = setup();
  native.isTauri.mockReturnValue(false);
  await expect(openFolderSelection(target)).rejects.toThrow("PROJECT_FOLDER_UNSUPPORTED");
  expect(native.invoke).not.toHaveBeenCalled();
});
