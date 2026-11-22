import { beforeEach, expect, it, vi } from "vitest";
import { newProject, useWorkspace } from "../../stores/workspace";
import { checkUncertainFolderWrite, handleFolderWriteResult } from "./folderWriteResult";

const mock = vi.hoisted(() => ({ read: vi.fn(), observe: vi.fn(), close: vi.fn() }));
vi.mock("../project-catalog/api", () => ({ readProjectSnapshot: mock.read }));
vi.mock("./commitObservation", () => ({ observeFolderCommit: mock.observe }));
vi.mock("./sourceApi", () => ({ closeFolderSource: mock.close }));

const source = { sourceId: "source", commitId: "base", taskScopeId: "task" };
beforeEach(() => {
  mock.close.mockResolvedValue(true);
  useWorkspace.getState().setProject(null);
  mock.read.mockReset();
  mock.observe.mockReset();
});

function fixture() {
  const previous = newProject("Original", "blank", "mm", "codex");
  useWorkspace.getState().openFolderProject(previous, source);
  const captured = useWorkspace.getState().sourceBinding;
  if (captured?.kind !== "folder") throw new Error("Expected an open folder");
  const result = {
    projectId: previous.id, sourceId: source.sourceId, baseCommitId: source.commitId,
    commitId: "next",
  };
  const next = { ...previous, name: "Committed" };
  const uncertain = { ...result, status: "indeterminate" as const, error: "Uncertain" };
  return { previous, captured, result, next, uncertain };
}

it("adopts a confirmed commit within the original opening", () => {
  const f = fixture();
  expect(handleFolderWriteResult(f.captured, { ...f.result, status: "committed", project: f.next }).status)
    .toBe("committed");
  expect(useWorkspace.getState().project).toBe(f.next);
  expect(useWorkspace.getState().sourceBinding).toMatchObject({
    openingId: f.captured.openingId, source: { ...source, commitId: "next" },
  });
});

it("leaves an uncertain write untouched until an explicit lookup", () => {
  const f = fixture();
  expect(handleFolderWriteResult(f.captured, f.uncertain)).toBe(f.uncertain);
  expect(mock.observe).not.toHaveBeenCalled();
  expect(useWorkspace.getState().project).toBe(f.previous);
});

it.each(["historical", "absent"])("leaves the model untouched if the attempted commit is %s", async (status) => {
  const f = fixture();
  mock.observe.mockResolvedValue({ status });
  expect(await checkUncertainFolderWrite(f.captured, f.uncertain)).toBe(status);
  expect(mock.read).not.toHaveBeenCalled();
  expect(useWorkspace.getState().project).toBe(f.previous);
});

it("adopts a proven current commit only after reading that exact snapshot", async () => {
  const f = fixture();
  mock.observe.mockResolvedValue({ status: "current" });
  mock.read.mockResolvedValue(f.next);
  expect(await checkUncertainFolderWrite(f.captured, f.uncertain)).toBe("current");
  expect(mock.read).toHaveBeenCalledWith(f.previous.id, { ...source, commitId: "next" });
  expect(useWorkspace.getState().project).toBe(f.next);
});

it("rejects a late snapshot after replacing the opening with an equal-ID project", async () => {
  const f = fixture();
  mock.observe.mockResolvedValue({ status: "current" });
  let finish!: (project: typeof f.next) => void;
  mock.read.mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
  const pending = checkUncertainFolderWrite(f.captured, f.uncertain);
  await vi.waitFor(() => expect(mock.read).toHaveBeenCalledOnce());
  const replacement = { ...f.previous, name: "Reopened" };
  useWorkspace.getState().openFolderProject(replacement, { ...source, sourceId: "replacement" });
  finish(f.next);
  expect(await pending).toBe("stale");
  expect(useWorkspace.getState().project).toBe(replacement);
  expect(await checkUncertainFolderWrite(f.captured, f.uncertain)).toBe("stale");
  expect(mock.observe).toHaveBeenCalledTimes(1);
});

it("does not read a stale commit when the same opening has advanced", async () => {
  const f = fixture();
  let finish!: (value: { status: string }) => void;
  mock.observe.mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
  const pending = checkUncertainFolderWrite(f.captured, f.uncertain);
  const replacement = { ...f.previous, name: "Newer commit" };
  expect(useWorkspace.getState().publishFolderSnapshot(
    replacement, f.captured, { ...source, commitId: "newer" },
  )).toBe(true);
  finish({ status: "current" });
  expect(await pending).toBe("stale");
  expect(mock.read).not.toHaveBeenCalled();
  expect(useWorkspace.getState().project).toBe(replacement);
});
