import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { newProject, useWorkspace } from "../../stores/workspace";
import { useFolderWriteAction } from "../project-sources/useFolderWriteAction";
import { useFolderWriteConfirmation } from "../project-sources/useFolderWriteConfirmation";
import { captureApply } from "./capturedApply";
import { writeFolderModel } from "./folderModelAction";

const mock = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", async original => ({
  ...await original<typeof import("@tauri-apps/api/core")>(),
  invoke: mock.invoke,
  isTauri: () => true,
}));

const source = {
  sourceId: "340f0083-c50f-4e15-b633-5cc4753ffb5c:1:8c45ccab-969f-4b94-a565-73db75eae0ef",
  commitId: "78b94346-f162-44e8-85c6-9cb64b8be739",
  taskScopeId: "340f0083-c50f-4e15-b633-5cc4753ffb5c",
};
const nextCommit = "7c6c3844-bdbe-4132-91ff-8e52a46a97ec";
const approval = "0fdca3bb-60ac-4014-99e9-28b8306687c7";
const content = { title: "Build model", description: "Check geometry", detail: "draft program" };

beforeEach(() => {
  useWorkspace.getState().setProject(null);
  mock.invoke.mockReset();
});

function setup() {
  const project = newProject("Folder", "blank", "mm", "codex");
  useWorkspace.getState().openFolderProject(project, source);
  const opening = useWorkspace.getState().sourceBinding;
  if (opening?.kind !== "folder") throw new Error("Expected a folder opening");
  const captured = captureApply(project, content.detail, "Update", opening.openingId);
  const onCommit = vi.fn(), reportError = vi.fn();
  const view = renderHook(() => {
    const active = useWorkspace(state => state.sourceBinding);
    return {
      action: useFolderWriteAction(active, onCommit, reportError),
      review: useFolderWriteConfirmation(active),
    };
  });
  const submit = () => {
    const { action, review } = view.result.current;
    return writeFolderModel(opening, captured, undefined, action.write, () => review.confirm(opening, content));
  };
  return { ...view, project, onCommit, reportError, submit };
}

it("reviews the original program, sends only its approval to apply and adopts the confirmed commit", async () => {
  const f = setup();
  const saved = { ...f.project, currentRevision: "6b0806c3-96db-42b2-b2e5-764a51a18907" };
  mock.invoke.mockResolvedValueOnce({ id: approval, requiresConfirmation: true });
  mock.invoke.mockResolvedValueOnce(undefined);
  mock.invoke.mockResolvedValueOnce({
    status: "committed", sourceId: source.sourceId, projectId: f.project.id,
    baseCommitId: source.commitId, commitId: nextCommit, project: saved,
  });

  let pending!: Promise<void>;
  act(() => { pending = f.submit(); });
  await waitFor(() => expect(f.result.current.review.pending).toEqual(content));
  expect(mock.invoke.mock.calls.map(([command]) => command)).toEqual(["request_permission"]);
  await act(async () => {
    f.result.current.review.decide(true);
    await pending;
  });
  expect(mock.invoke.mock.calls.map(([command]) => command)).toEqual(["request_permission", "resolve_permission", "apply_program"]);
  expect(mock.invoke.mock.calls[0]?.[1]).toMatchObject({
    projectId: f.project.id, sourceId: source.sourceId, expectedCommitId: source.commitId,
    folderIntent: { kind: "model", program: content.detail, expectedRevision: null },
  });
  expect(mock.invoke.mock.calls[2]?.[1]).toEqual({
    projectId: f.project.id,
    folderWrite: { sourceId: source.sourceId, expectedCommitId: source.commitId, approvalId: approval },
  });
  expect(useWorkspace.getState().project).toEqual(saved);
  expect(useWorkspace.getState().sourceBinding).toMatchObject({ kind: "folder", source: { ...source, commitId: nextCommit } });
  expect(f.onCommit).toHaveBeenCalledOnce();
  expect(f.reportError).not.toHaveBeenCalled();
});

it("discards the approval after reopening another folder with the same project ID", async () => {
  const f = setup();
  mock.invoke.mockResolvedValueOnce({ id: approval, requiresConfirmation: true });
  let pending!: Promise<void>;
  act(() => { pending = f.submit(); });
  await waitFor(() => expect(f.result.current.review.pending).toEqual(content));
  act(() => useWorkspace.getState().openFolderProject(f.project, { ...source, sourceId: "other-opening" }));
  await act(async () => { await pending; });
  expect(mock.invoke.mock.calls.map(([command]) => command)).toEqual(["request_permission", "close_project_source", "resolve_permission"]);
  expect(mock.invoke.mock.calls[2]?.[1]).toMatchObject({ id: approval, allow: false, sourceId: source.sourceId });
  expect(f.result.current.review.pending).toBeNull();
  expect(f.onCommit).not.toHaveBeenCalled();
});
