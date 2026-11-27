import { beforeEach, expect, it, vi } from "vitest";
import { newProject, useWorkspace } from "../../stores/workspace";
import { runFolderWrite } from "./runFolderWrite";

const mock = vi.hoisted(() => ({ request: vi.fn(), resolve: vi.fn(), execute: vi.fn(), close: vi.fn() }));
vi.mock("../../lib/api", () => ({
  requestNativePermission: mock.request,
  resolveNativePermission: mock.resolve,
}));
vi.mock("./writeApi", () => ({ executeFolderWrite: mock.execute }));
vi.mock("./sourceApi", () => ({ closeFolderSource: mock.close }));

const source = { sourceId: "source", commitId: "base", taskScopeId: "task" };
const intent = { kind: "history", direction: "undo", expectedRevision: null } as const;

beforeEach(() => {
  mock.close.mockResolvedValue(true);
  useWorkspace.getState().setProject(null);
  mock.request.mockReset();
  mock.resolve.mockReset().mockResolvedValue(undefined);
  mock.execute.mockReset();
});

function fixture(requiresConfirmation: boolean) {
  const project = newProject("Original", "blank", "mm", "codex");
  useWorkspace.getState().openFolderProject(project, source);
  const captured = useWorkspace.getState().sourceBinding;
  if (captured?.kind !== "folder") throw new Error("Expected an open folder");
  const ticket = { id: "approval", projectId: project.id, source, requiresConfirmation };
  mock.request.mockResolvedValue(ticket);
  mock.execute.mockResolvedValue({
    status: "committed", projectId: project.id, sourceId: source.sourceId,
    baseCommitId: source.commitId, commitId: "next", project: { ...project, name: "Updated" },
  });
  return { project, captured, ticket };
}

it.each([true, false])("executes only the retained Undo after approval (dialog: %s)", async (requiresConfirmation) => {
  const f = fixture(requiresConfirmation);
  const confirm = vi.fn(async () => true);
  expect(await runFolderWrite(f.captured, intent, "Undo", confirm)).toMatchObject({ status: "committed" });
  expect(mock.request).toHaveBeenCalledExactlyOnceWith(f.project.id, "modify_project", "Undo", { source, intent });
  expect(mock.execute).toHaveBeenCalledExactlyOnceWith(f.ticket, "undo");
  expect(confirm).toHaveBeenCalledTimes(requiresConfirmation ? 1 : 0);
  expect(mock.resolve).toHaveBeenCalledTimes(requiresConfirmation ? 1 : 0);
  if (requiresConfirmation) expect(mock.resolve).toHaveBeenCalledWith(f.ticket.id, true, f.ticket);
  expect(useWorkspace.getState().project?.name).toBe("Updated");
});

it("records denial without executing the approved intent", async () => {
  const f = fixture(true);
  expect(await runFolderWrite(f.captured, intent, "Undo", async () => false)).toEqual({ status: "denied" });
  expect(mock.resolve).toHaveBeenCalledExactlyOnceWith(f.ticket.id, false, f.ticket);
  expect(mock.execute).not.toHaveBeenCalled();
});

it("discards approval if the original opening was replaced during confirmation", async () => {
  const f = fixture(true);
  const result = await runFolderWrite(f.captured, intent, "Undo", async () => {
    useWorkspace.getState().openFolderProject(f.project, { ...source, sourceId: "replacement" });
    return true;
  });
  expect(result).toEqual({ status: "stale" });
  expect(mock.resolve).toHaveBeenCalledExactlyOnceWith(f.ticket.id, false, f.ticket);
  expect(mock.execute).not.toHaveBeenCalled();
});

it("drops the result if the opening changes while execution is pending", async () => {
  const f = fixture(false);
  mock.execute.mockImplementationOnce(async () => {
    useWorkspace.getState().openFolderProject(f.project, { ...source, sourceId: "replacement" });
    return { status: "noOp", projectId: f.project.id, sourceId: source.sourceId, baseCommitId: source.commitId, project: f.project };
  });
  expect(await runFolderWrite(f.captured, intent, "Undo", async () => true)).toEqual({ status: "stale" });
  expect(useWorkspace.getState().project).toBe(f.project);
});
