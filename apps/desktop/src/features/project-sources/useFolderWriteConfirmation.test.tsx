import { act, renderHook } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { newProject, useWorkspace } from "../../stores/workspace";
import type { ProjectSourceBinding } from "./sourceBinding";
import { useFolderWriteConfirmation } from "./useFolderWriteConfirmation";

vi.mock("./sourceApi", () => ({ closeFolderSource: vi.fn(async () => true) }));
const source = { sourceId: "first", commitId: "head", taskScopeId: "tasks" };
beforeEach(() => { useWorkspace.getState().setProject(null); });

function fixture() {
  const project = newProject("Folder", "blank", "mm", "codex");
  useWorkspace.getState().openFolderProject(project, source);
  const opening = useWorkspace.getState().sourceBinding;
  if (opening?.kind !== "folder") throw new Error("Expected a folder opening");
  const view = renderHook(({ binding }: { binding: ProjectSourceBinding | null }) =>
    useFolderWriteConfirmation(binding),
    { initialProps: { binding: opening as ProjectSourceBinding | null } },
  );
  return { project, opening, ...view };
}

const content = { title: "Model edit", description: "Review geometry", detail: "original" };

it.each([true, false])("resolves the review dialog decision as %s", async (allow) => {
  const f = fixture();
  let decision!: Promise<boolean>;
  act(() => { decision = f.result.current.confirm(f.opening, content); });
  expect(f.result.current.pending?.detail).toBe("original");
  act(() => f.result.current.decide(allow));
  expect(await decision).toBe(allow);
  expect(f.result.current.pending).toBeNull();
});

it("rejects the old review when the same project is reopened in another folder", async () => {
  const f = fixture();
  let decision!: Promise<boolean>;
  act(() => { decision = f.result.current.confirm(f.opening, content); });
  useWorkspace.getState().openFolderProject(f.project, { ...source, sourceId: "second" });
  f.rerender({ binding: useWorkspace.getState().sourceBinding });
  expect(await decision).toBe(false);
  expect(f.result.current.pending).toBeNull();
});
