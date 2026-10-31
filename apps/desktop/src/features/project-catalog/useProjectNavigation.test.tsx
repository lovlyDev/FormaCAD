import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { newProject, useWorkspace } from "../../stores/workspace";
import { defaults, type Project } from "../../types";
import { useProjectNavigation } from "./useProjectNavigation";
const mock = vi.hoisted(() => ({ read: vi.fn() }));
vi.mock("./api", () => ({ readProjectSnapshot: mock.read }));
function project(name: string) { const value = newProject(name, "blank", "mm", "codex"); value.id = name; return value; }
beforeEach(() => { mock.read.mockReset(); useWorkspace.getState().setProject(null); useWorkspace.getState().setBusy(false); });

it("preserves the active workspace when a different target cannot be read", async () => {
  const active = project("active"), navigate = vi.fn(), error = vi.fn();
  useWorkspace.getState().setProject(active);
  mock.read.mockRejectedValue(new Error("PROJECT_SNAPSHOT_RECOVERY_REQUIRED"));
  const view = renderHook(() => useProjectNavigation("pending", navigate, error));
  await waitFor(() => expect(error).toHaveBeenCalledTimes(1));
  expect(useWorkspace.getState().project).toBe(active);
  expect(navigate).toHaveBeenCalledWith("/project/active");
  view.unmount();
});

it("rejects an older project result even when both projects share revision and body IDs", async () => {
  const a = project("A"), b = project("B"), navigate = vi.fn(), error = vi.fn();
  // Navigation treats the model program as opaque; this is an identity fixture,
  // not a kernel-valid CAD document or a geometry acceptance test.
  const revision = { id: "shared-revision", parent: null, createdAt: a.createdAt,
    prompt: "identity fixture", parameters: defaults, program: '{"bodyId":"shared-body"}' };
  a.revisions = [revision]; b.revisions = [revision];
  a.currentRevision = revision.id; b.currentRevision = revision.id;
  const finish = new Map<string, (value: Project) => void>();
  mock.read.mockImplementation((id: string) => new Promise<Project>(resolve => { finish.set(id, resolve); }));
  const view = renderHook(({ id }) => useProjectNavigation(id, navigate, error), { initialProps: { id: "A" } });
  view.rerender({ id: "B" });
  await act(async () => { finish.get("B")!(b); await Promise.resolve(); });
  await act(async () => { finish.get("A")!(a); await Promise.resolve(); });
  expect(useWorkspace.getState().project).toBe(b);
  expect(error).not.toHaveBeenCalled();
  view.unmount();
});

it("opens a fresh target instead of adopting the card snapshot", async () => {
  const old = project("A"), latest = { ...old, name: "fresh metadata" }, navigate = vi.fn(), error = vi.fn();
  mock.read.mockResolvedValue(latest);
  const view = renderHook(() => useProjectNavigation(undefined, navigate, error));
  await act(async () => { await view.result.current(old); });
  expect(mock.read).toHaveBeenCalledWith("A");
  expect(useWorkspace.getState().project).toBe(latest);
  expect(navigate).toHaveBeenCalledWith("/project/A");
  view.unmount();
});

it("cannot adopt an explicit opening after the hook unmounts", async () => {
  const active = project("A"), next = project("B"), navigate = vi.fn(), error = vi.fn();
  useWorkspace.getState().setProject(active);
  let finish!: (value: Project) => void;
  mock.read.mockImplementation(() => new Promise<Project>(resolve => { finish = resolve; }));
  const view = renderHook(() => useProjectNavigation("A", navigate, error));
  let pending!: Promise<void>;
  act(() => { pending = view.result.current(next); });
  view.unmount();
  await act(async () => { finish(next); await pending; });
  expect(useWorkspace.getState().project).toBe(active);
  expect(navigate).not.toHaveBeenCalled();
});

it("does not replace a workspace when work starts during a direct-route read", async () => {
  const active = project("A"), next = project("B"), navigate = vi.fn(), error = vi.fn();
  useWorkspace.getState().setProject(active);
  let finish!: (value: Project) => void;
  mock.read.mockImplementation(() => new Promise<Project>(resolve => { finish = resolve; }));
  const view = renderHook(() => useProjectNavigation("B", navigate, error));
  useWorkspace.getState().setBusy(true);
  await act(async () => { finish(next); await Promise.resolve(); });
  expect(useWorkspace.getState().project).toBe(active);
  expect(error).toHaveBeenCalledTimes(1);
  expect(navigate).toHaveBeenCalledWith("/project/A");
  view.unmount();
});
