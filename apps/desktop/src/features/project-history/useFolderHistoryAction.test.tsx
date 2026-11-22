import { act, renderHook } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { newProject, useWorkspace } from "../../stores/workspace";
import { folderHistoryAction } from "./folderHistoryAction";
import { useFolderWriteAction } from "../project-sources/useFolderWriteAction";
import { writeFolderModel } from "../model-apply/folderModelAction";
import { captureApply } from "../model-apply/capturedApply";

const mock = vi.hoisted(() => ({ run: vi.fn(), check: vi.fn(), close: vi.fn() }));
vi.mock("../project-sources/runFolderWrite", () => ({ runFolderWrite: mock.run }));
vi.mock("../project-sources/folderWriteResult", async (original) => ({
  ...await original<typeof import("../project-sources/folderWriteResult")>(),
  checkUncertainFolderWrite: mock.check,
}));
vi.mock("../project-sources/sourceApi", () => ({ closeFolderSource: mock.close }));

const source = { sourceId: "source", commitId: "base", taskScopeId: "tasks" };

beforeEach(() => {
  mock.close.mockResolvedValue(true);
  useWorkspace.getState().setProject(null);
  mock.run.mockReset();
  mock.check.mockReset();
});

function fixture() {
  const project = newProject("Folder", "blank", "mm", "codex");
  useWorkspace.getState().openFolderProject(project, source);
  const binding = useWorkspace.getState().sourceBinding!;
  const confirm = vi.fn(async () => true);
  const refresh = vi.fn();
  const report = vi.fn();
  const hook = renderHook(({ opening }) => {
    const write = useFolderWriteAction(opening, refresh, report);
    return { ...write, act: folderHistoryAction(opening, write.write, confirm) };
  }, {
    initialProps: { opening: binding },
  });
  return { project, binding, ...hook, refresh, report };
}

it("locks the history action before the first host response", async () => {
  const f = fixture();
  let finish!: (value: { status: "committed" }) => void;
  mock.run.mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; }));
  await act(async () => {
    const pending = f.result.current.act("undo");
    await f.result.current.act("redo");
    finish({ status: "committed" });
    await pending;
  });
  expect(mock.run).toHaveBeenCalledTimes(1);
  expect(mock.run.mock.calls[0]?.slice(0, 2)).toEqual([f.binding, {
    kind: "history", direction: "undo", expectedRevision: f.project.currentRevision,
  }]);
  expect(f.refresh).toHaveBeenCalledOnce();
  expect(f.result.current.busy).toBe(false);
});

it("blocks a model apply while a history publication is unresolved", async () => {
  const f = fixture();
  if (f.binding.kind !== "folder") throw new Error("Expected a folder opening");
  const opening = f.binding;
  const captured = captureApply(f.project, "program", "change", opening.openingId);
  const confirm = vi.fn(async () => true);
  const plan = { assumptions: [], dimensions: [], affectedBodyIds: [], expectedChecks: [] };
  mock.run.mockResolvedValueOnce({
    status: "indeterminate", projectId: f.project.id, sourceId: source.sourceId,
    baseCommitId: source.commitId, commitId: "attempt", error: "Uncertain",
  });
  await act(() => f.result.current.act("undo"));
  const submit = () => writeFolderModel(opening, captured, plan, f.result.current.write, confirm);
  await act(submit);
  expect(mock.run).toHaveBeenCalledTimes(1);

  mock.check.mockResolvedValueOnce("current");
  await act(() => f.result.current.check());
  mock.run.mockResolvedValueOnce({ status: "noOp" });
  await act(submit);
  expect(mock.run).toHaveBeenCalledTimes(2);
  expect(mock.run.mock.calls[1]).toEqual([opening, {
    kind: "model", program: "program", prompt: "change", expectedRevision: null, reviewPlan: plan,
  }, "program", confirm]);
});

it("refuses a captured model after a same-ID replacement folder", async () => {
  const f = fixture();
  if (f.binding.kind !== "folder") throw new Error("Expected a folder opening");
  const opening = f.binding;
  const captured = captureApply(f.project, "program", "change", opening.openingId);
  act(() => useWorkspace.getState().openFolderProject(f.project, { ...source, sourceId: "other" }));
  await expect(writeFolderModel(opening, captured, undefined, f.result.current.write, async () => true))
    .rejects.toThrow("MODEL_APPLY_BASE_CHANGED");
  expect(mock.run).not.toHaveBeenCalled();
});

it("keeps an unresolved history write blocked and checks it only on request", async () => {
  const f = fixture();
  mock.run.mockResolvedValue({
    status: "indeterminate", projectId: f.project.id, sourceId: source.sourceId,
    baseCommitId: source.commitId, commitId: "attempt", error: "Uncertain",
  });
  await act(() => f.result.current.act("undo"));
  await act(() => f.result.current.act("redo"));
  expect(mock.run).toHaveBeenCalledTimes(1);
  expect(mock.check).not.toHaveBeenCalled();
  mock.check.mockResolvedValueOnce("absent").mockResolvedValueOnce("current");
  await act(() => f.result.current.check());
  expect(f.result.current.uncertain?.observation).toBe("absent");
  await act(() => f.result.current.check());
  expect(f.result.current.uncertain).toBeNull();
  expect(f.refresh).toHaveBeenCalledOnce();
});

it("does not report a late failure against a replacement folder", async () => {
  const f = fixture();
  mock.run.mockImplementationOnce(async () => {
    useWorkspace.getState().openFolderProject(f.project, { ...source, sourceId: "other" });
    throw new Error("old failure");
  });
  await act(() => f.result.current.act("undo"));
  expect(f.report).not.toHaveBeenCalled();
  expect(f.refresh).not.toHaveBeenCalled();
});
