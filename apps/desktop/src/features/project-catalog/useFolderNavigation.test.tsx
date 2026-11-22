import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { newProject, useWorkspace } from "../../stores/workspace";
import { useProjectNavigation } from "./useProjectNavigation";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), read: vi.fn() }));
vi.mock("@tauri-apps/api/core", async original => ({
  ...await original<typeof import("@tauri-apps/api/core")>(),
  invoke: mocks.invoke,
  isTauri: () => true,
}));
vi.mock("./api", () => ({ readProjectSnapshot: mocks.read }));

const managed = newProject("Managed", "blank", "mm", "codex");
const folder = { ...managed, name: "From the folder" };
const source = { sourceId: "window:1:nonce", commitId: "commit", taskScopeId: "tasks" };

beforeEach(() => {
  useWorkspace.getState().setProject(null);
  useWorkspace.getState().setBusy(false);
  vi.resetAllMocks();
});

it("installs the selected folder and keeps it active when its route is entered", async () => {
  const navigate = vi.fn(), error = vi.fn();
  const view = renderHook(
    ({ id }) => useProjectNavigation(id, navigate, error, id !== undefined),
    { initialProps: { id: undefined as string | undefined } },
  );
  mocks.invoke.mockResolvedValueOnce({ project: folder, ...source });
  await act(async () => { await view.result.current.openFolder(); });
  expect(navigate).toHaveBeenCalledWith("/folder/" + folder.id);
  view.rerender({ id: folder.id });
  expect(useWorkspace.getState().project).toBe(folder);
  expect(useWorkspace.getState().sourceBinding).toMatchObject({ kind: "folder", source });
  expect(mocks.read).not.toHaveBeenCalled();
});

it("redirects a reloaded folder route home even if a managed project matches the ID", () => {
  const navigate = vi.fn(), error = vi.fn();
  useWorkspace.getState().openManagedProject(managed);
  renderHook(() => useProjectNavigation(folder.id, navigate, error, true));
  expect(navigate).toHaveBeenCalledWith("/");
  expect(mocks.read).not.toHaveBeenCalled();
});

it("reads the managed snapshot when navigating from a folder to the same managed ID", async () => {
  const navigate = vi.fn(), error = vi.fn();
  useWorkspace.getState().openFolderProject(folder, source);
  mocks.read.mockResolvedValueOnce(managed);
  renderHook(() => useProjectNavigation(folder.id, navigate, error));
  await waitFor(() => expect(useWorkspace.getState().project).toBe(managed));
  expect(useWorkspace.getState().sourceBinding).toMatchObject({ kind: "managed" });
  expect(mocks.read).toHaveBeenCalledWith(managed.id);
  expect(mocks.invoke).toHaveBeenCalledWith("close_project_source", { sourceId: source.sourceId });
});

it("coalesces picker clicks and releases the selection if a managed card wins navigation", async () => {
  const navigate = vi.fn(), error = vi.fn();
  let finish!: (selection: unknown) => void;
  mocks.invoke.mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
  mocks.read.mockResolvedValueOnce(managed);
  const view = renderHook(() => useProjectNavigation(undefined, navigate, error));
  let pending!: Promise<void>;
  act(() => { pending = view.result.current.openFolder(); });
  await act(async () => {
    await view.result.current.openFolder();
    await view.result.current.openProject(managed);
    finish({ project: folder, ...source });
    await pending;
  });
  expect(navigate.mock.calls).toEqual([["/project/" + managed.id]]);
  expect(useWorkspace.getState().project).toBe(managed);
  expect(mocks.invoke.mock.calls).toEqual([
    ["open_project_folder"],
    ["close_project_source", { sourceId: source.sourceId }],
  ]);
});
